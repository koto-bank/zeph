[![Build Status](https://travis-ci.org/koto-bank/zeph.svg?branch=master)](https://travis-ci.org/koto-bank/zeph)
# Zeph

A \*booru written in Rust with Iron and PostgreSQL (though there is also an abandoned SQLite module..)

Also, some Postgres modules are required: `citext`, `smlar` and `hstore`.

# Features

* Synchronization with some other booru's (e.g. Gelbooru, Konachan) in `src/db`
    * Syncing is controlled with console in `/admin`
* Search with tags, partial tags, uploader, etc. (optionally synced booru's with `from:booru_name`) <!-- TODO: probably document it? -->
* Sort images by ascending/descending of score/id (e.g. `sort:asc:score`)
* HTTP API (it is there, but not well documented) <!-- TODO: document it.. -->
* (Kind of) configurable
    * There is some basic configuration to do in `Config.toml`,
    `images-directory` is where the pictures are stored, `postgres-login` and `postgres-password` are used to connect to the database.
    * `contact-email` is optional and, if set, will be shown on `/about` page
    * `admin-username` is account to access `/admin` panel
* Users and registration, passwords are encrypted with scrypt. Users can vote for images (not for `sync`ed, though, because their score is based on the original score and updates when you `sync`)
* Similiar images, based on tags.

# Running

Currently in works only on nightly rust.

Just run it with `cargo run --release` or build it and place the compiled binary into the crate root.

# Screenshots

(All images belong to their respective authors)

![Search page](/screenshots/screenshot_main.png?raw=true)
![Image page](/screenshots/screenshot_show.png?raw=true)

# Contributing

Contributions are hightly appreciated! Open a PR if you have features to add
