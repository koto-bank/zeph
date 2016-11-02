//! Украдено отсюда: https://github.com/joel-wright/rust-parallel-download

use super::hyper::header::{Headers, Range, ContentLength, ByteRangeSpec};
use super::hyper::{self,Client};

use std::cmp;
use std::collections::LinkedList;
use std::error;
use std::error::Error;
use std::fmt;
use std::io;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub enum DownloadError {
    Http(hyper::error::Error),
    Fail(String)
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DownloadError::Http(ref err) => write!(
                f, "ParallelDownload HTTP error: {}", err),
                DownloadError::Fail(ref s) => write!(
                    f, "ParallelDownload Fail: {}", s),
        }
    }
}

impl error::Error for DownloadError {
    fn description(&self) -> &str {
        match *self {
            DownloadError::Http(ref err) => err.description(),
            DownloadError::Fail(ref s) => s,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DownloadError::Http(ref err) => Some(err),
            DownloadError::Fail(_) => None,
        }
    }
}

pub struct Download {
    client: Client,
    url: String,
    start: u64,
    size: u32,            // Limit to 4G in a single range request
    bytes_read: u32,      // index of last byte for which a read has been scheduled
    read_size: u32,       // size of individual reads from the socket
    recv: Receiver<()>,   // to allow responsive shutdown
}

impl Download {
    pub fn new(url: String, start: u64, size: u32, rx: Receiver<()>) -> Download {
        let client = Client::new();
        Download {
            client: client,
            url: url,
            start: start,
            size: size,
            bytes_read: 0,
            read_size: 1024*1024,
            recv: rx
        }
    }

    #[allow(dead_code)]
    pub fn set_read_size(&mut self, size: u32) {
        self.read_size = size;
    }

    pub fn download(&mut self) -> Result<Vec<u8>, DownloadError> {
        // Start performing the actual download
        //
        // Make the request, then read into the local
        // storage buffer. During the download monitor
        // for calls to kill.
        let mut buffer: Vec<u8> = vec![0; self.size as usize];  // Vec::with_capacity((self.size as usize));
        let mut headers = Headers::new();
        let r_start = self.start;
        let r_end = r_start + self.size as u64 - 1;
        headers.set(
            Range::Bytes(
                vec![ByteRangeSpec::FromTo(r_start, r_end)]
                )
            );
        let body = self.client.get(&self.url).headers(headers).send();
        let mut res = match body {
            Ok(res) => res,
            Err(e) => {
                // println!("{:?}", e);
                return Err(DownloadError::Http(e))
            }
        };

        // start reading the request, with checks for abort
        while self.bytes_read < self.size {
            match self.recv.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    // Any communication means stop
                    // TODO: close connection?
                    break;
                }
                Err(TryRecvError::Empty) => {
                    // The actual work goes here
                    let b_start = self.bytes_read;
                    // println!("start byte: {:?}, in size: {:?}, trying to read {:?}", b_start, buffer.len(), b_size);
                    let mut range_buffer: &mut [u8] = &mut buffer[b_start as usize .. self.size as usize];
                    let _r_size = match res.read(&mut range_buffer) {
                        Ok(rs) => rs,
                        Err(_) => 0
                    };  // TODO: range might be smaller
                    self.bytes_read += _r_size as u32;
                    if _r_size == 0 && self.bytes_read < self.size {
                        return Err(DownloadError::Fail(
                                format!(
                                    "Got no more bytes after reading {}, but expected {}!",
                                    self.bytes_read,
                                    self.size
                                    ))
                                  )
                    };
                }
            };
        }

        if self.bytes_read < self.size {
            // println!("{:?}", "truncating smaller vec");
            buffer.truncate(self.bytes_read as usize)
        }
        // println!("download of {:?} bytes complete", self.size);
        Ok(buffer)  // TODO: make sure we return a buffer of the right size!
    }
}

pub struct ParallelDownload {
    url: String,
    client: Client,
    downloaders: LinkedList<JoinHandle<Result<Vec<u8>, DownloadError>>>,
    downloader_kill_channels: LinkedList<Sender<()>>,
    current_vec: Option<Vec<u8>>,
    chunk_size: u32,
    thread_count: u32,
    next_start_byte: u64,
    next_read_offset: u64,
    content_length: u64
}

impl ParallelDownload {
    pub fn new(url: String, chunk_size: u32, thread_count: u32) -> ParallelDownload {
        let client = Client::new();
        let downloader_list:
            LinkedList<JoinHandle<Result<Vec<u8>, DownloadError>>> = LinkedList::new();
        let kill_channel_list: LinkedList<Sender<()>> = LinkedList::new();
        let _cs = if chunk_size < 1 {1} else {chunk_size};
        let _tc = if thread_count < 1 {1} else {thread_count};

        ParallelDownload {
            url: url,
            client: client,
            downloaders: downloader_list,
            downloader_kill_channels: kill_channel_list,
            current_vec: None,
            chunk_size: _cs*1000*1000,
            thread_count: _tc,
            next_start_byte: 0,
            next_read_offset: 0,
            content_length: 0
        }
    }

    pub fn kill(&mut self) {
        for k in &self.downloader_kill_channels {
            match k.send(()) {
                // TODO: Handle this properly
                Ok(_) => {},
                Err(_) => {}
            };
        }
    }

    fn try_start_thread(&mut self, start_byte: u64, content_length: u64)
        -> Option<u64> {
            // Attempt to create a new download thread for a given
            // start byte.
            //
            // Some(end_byte) indicates that a thread has been created
            // None indicates that the content_length has been reached
            if start_byte >= content_length {
                return None;
            }

            // println!("{:?}", "new thread...");

            let next_start_byte: u64 = {
                let mut _b: u64 = start_byte + self.chunk_size as u64;
                if _b > content_length {
                    _b = content_length
                };
                _b
            };

            let _u = self.url.clone();
            let _cl = self.content_length;
            let _s = cmp::min(self.chunk_size as u64, _cl - start_byte) as u32;
            let (tx, rx) = channel();
            let mut _d = Download::new(_u, start_byte, _s, rx);
            self.downloader_kill_channels.push_back(tx);
            let _dl_thread = thread::spawn(move || {
                // Start the download, and return the full buffer when complete
                _d.download()
            });
            self.downloaders.push_back(_dl_thread);

            Some(next_start_byte)  // return the next start byte
        }

    pub fn start_download(&mut self) -> Result<(), DownloadError> {
        // head to get size and range req support
        let head_resp = try!(match self.client.head(&self.url).send() {
            Ok(r) => Ok(r),
            Err(e) => Err(DownloadError::Http(e))
        });
        // get size from headers
        self.content_length = match head_resp.headers.get() {
            Some(&ContentLength(ref _cl)) => *_cl,  // : & u64
            None => return Err(DownloadError::Fail(
                    String::from("No content length found")
                    ))
        };
        //println!("{:?}", self.content_length);

        // start filling the thread pools and downloads
        let mut thread_count = 0;
        let mut next_start_byte: u64 = 0;
        let cl = self.content_length;
        while (thread_count < self.thread_count) && (next_start_byte < cl) {
            let start_byte: u64 = next_start_byte;
            next_start_byte = match self.try_start_thread(start_byte, cl) {
                None => cl,
                Some(_u) => {
                    thread_count += 1;
                    _u
                }
            };
        }

        self.next_start_byte = next_start_byte;
        Ok(())
    }
}

impl Read for ParallelDownload {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        // read buf.len() bytes into buf
        // println!("{:?}", self.current_vec);
        match self.current_vec {
            None => {
                // try to get the next buffer
                let dt_handle = match self.downloaders.pop_front() {
                    None => return Ok(0),
                    Some(_dt) => _dt
                };
                let d_result: Vec<u8> = match dt_handle.join() {
                    Err(_) => {
                        self.kill();
                        //println!("{}", "oh no!");
                        return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "ParallelDownload: Failed to join internal download thread"
                                ))
                    },
                    Ok(buf) => {
                        // If we've got a result, it's time to kick off
                        // a new thread (if possible)
                        let cl = self.content_length;
                        let nsb = self.next_start_byte;
                        match self.try_start_thread(nsb, cl) {
                            None => (),
                            Some(nsb) => {
                                self.next_start_byte = nsb
                            }
                        };
                        match buf {
                            Ok(v) => v,
                            Err(e) => {
                                //println!("{}", e);
                                return Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        e.description()
                                        ))
                            }
                        }
                    }
                };
                self.current_vec = Some(d_result);
            },
            _ => ()
        };

        let (complete, len) = match self.current_vec {
            Some(ref v) => {
                // println!("{}", v.len());

                // Read as much as possible from the current Vec
                let nro = self.next_read_offset;
                // The max we can read is the size of the data remaining in
                // the current Vec, or the length of the supplied buffer
                let _v_len = v.len() - nro as usize;
                let _len = cmp::min(buf.len(), _v_len);
                // Always copy into the beginning of the supplied buffer
                let mut buf_sl = &mut buf[.. _len];
                let new_nro = nro + _len as u64;
                // Copy the data from the current Vec into the supplied buffer
                let current_buf_sl = &v[nro as usize .. new_nro as usize];
                buf_sl.clone_from_slice(current_buf_sl);

                if new_nro >= v.len() as u64 {
                    // println!("{:?}", "Finished reading buffer");
                    // Reset the offset to 0 and return that we have completed
                    // reading from the current buffer
                    self.next_read_offset = 0;
                    (true, _len)
                } else {
                    self.next_read_offset = new_nro;
                    (false, _len)
                }
            },
            _ => return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "ParallelDownload: We should never reach here!"
                    )),
        };

        if complete {
            // Setting the current Vec to None will trigger grabbing the next
            // download chunk on the next read
            self.current_vec = None;
        }

        // Make sure we return the correct number of bytes read
        Ok(len)
    }
}
