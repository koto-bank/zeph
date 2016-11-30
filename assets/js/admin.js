var CURR_LOG = [];

function httpGetAsync(theUrl, callback) {
    var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4 && xmlHttp.status == 200) { callback(xmlHttp.responseText); }
    }
    xmlHttp.open("GET", theUrl, true);
    xmlHttp.send(null);
}

function httpPostAsync(theUrl, params/*, callback*/) {
    var body = "";
    for (var p in params) {
        body += encodeURIComponent(p) + "=" + encodeURIComponent(params[p]) + "&"
    }

    var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("POST", theUrl, true);
    xmlHttp.send(body);
} // FIXME: And here it says "Element not found". What element..?

function sendCommand(frm) {
    httpPostAsync("/admin", { command: frm.comm.value });
    frm.comm.value = ""
}

function getLog() {
    var bl = document.getElementById("log-block");

    httpGetAsync("/log", function(text) {
        var body = JSON.parse(text);
        if (CURR_LOG.length != body.length) {
            bl.innerHTML = "";
            body.forEach(function(l) {
                var s = document.createTextNode(l);
                var br = document.createElement("br");
                bl.appendChild(s);
                bl.appendChild(br);
                bl.scrollTop = bl.scrollHeight;
            });
            CURR_LOG = body
        }
    });
}

window.setInterval(getLog, 2000);
