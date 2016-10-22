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
        console.log(body);
        var tags_block = document.getElementById("tags");
        document.getElementById("id").textContent = "#" + body["id"];
        if (body["original_link"] != " ") {
            var l = document.createElement("a");
            l.href = body["original_link"];
            l.textContent = "Original page";
            tags_block.appendChild(l);
            tags_block.appendChild(document.createElement("br"));
        }

        if (body["rating"] != " ") {
            var l = document.createElement("a");
            l.href = "/search?q=rating:" + body["rating"];
            l.textContent = "rating:" + body["rating"];
            tags_block.appendChild(l);
        }

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
