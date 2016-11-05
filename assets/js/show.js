function httpGetAsync(theUrl, callback) {
    var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4 && xmlHttp.status == 200) { callback(xmlHttp.responseText); }
    }
    xmlHttp.open("GET", theUrl, true);
    xmlHttp.send(null);
}

function load(){
    var reg = /show\/(\d+)/;
    httpGetAsync(window.location.pathname.replace("show","get_image"), function(text){
        var body = JSON.parse(text);
        var tags_block = document.getElementById("tags");
        var image_info = document.getElementById("image-info");
        document.getElementById("id").textContent = "#" + body["id"];
        if (body["original_link"] !== null) {
            var l = document.createElement("a");
            l.href = body["original_link"];
            l.textContent = "Original page";
            image_info.appendChild(l);
            image_info.appendChild(document.createElement("br"));
        }

        if (body["rating"] !== null) {
            var l = document.createElement("a");
            l.href = "/search?q=rating:" + body["rating"];
            l.textContent = "rating:" + body["rating"];
            image_info.appendChild(l);
            image_info.appendChild(document.createElement("br"));
        }

        if (body["got_from"] !== null) {
            var l = document.createElement("a");
            l.href = "/search?q=from:" + body["got_from"];
            l.textContent = "from:" + body["got_from"];
            image_info.appendChild(l);
            image_info.appendChild(document.createElement("br"));
        }

        if (body["uploader"] !== null) {
            var l = document.createElement("a");
            l.href = "/search?q=uploader:" + body["uploader"];
            l.textContent = "uploader:" + body["uploader"];
            image_info.appendChild(l);
            image_info.appendChild(document.createElement("br"));
        }

        var score = document.createTextNode("Score: " + body["score"]);
        image_info.appendChild(score);
        image_info.appendChild(document.createElement("br"));

        httpGetAsync("/user_status", function(text){
            var userstatus = JSON.parse(text);
            console.log(userstatus);
            if (userstatus["logined"] == true && userstatus["name"] == body["uploader"]) {
                var l = document.createElement("a");
                l.href = window.location.pathname.replace("show","delete");
                l.textContent = "Delete image";
                image_info.appendChild(l);
                image_info.appendChild(document.createElement("br"));
            }
        });

        body["tags"].forEach(function(tag) {
            tags_block.appendChild(document.createElement("br"));
            var link = document.createElement("a");
            link.textContent = tag;
            link.href = "/search?q=" + tag;
            tags_block.appendChild(link);
        });

        var image_block = document.getElementById("image-block");
        image_block.parentNode.href = "/images/" + body["name"];
        image_block.src = "/images/" + body["name"];
    });
}

window.onload = function() {
    load();
}
