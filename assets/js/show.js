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
    var id = reg.exec(window.location.pathname)[1];

    httpGetAsync("/get_image/" + id, function(text){
        var body = JSON.parse(text);
        var tags_block = document.getElementById("tags");
        var image_info = document.getElementById("image-info");
        document.getElementById("id").textContent = "#" + body["id"];
        document.getElementsByTagName("title")[0].textContent = "Zeph - " + body["tags"].join(" ");

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

        var vote_up_a = document.createElement("a");
        vote_up_a.href = "#";
        vote_up_a.style.display = "inline-block";
        var vote_down_a = document.createElement("a");
        vote_down_a.href = "#";
        vote_down_a.style.display = "inline-block";

        var vote_area = document.createElement("div");

        var score = document.createElement("div");
        score.textContent = "Score: " + body["score"];

        vote_area.appendChild(score);

        image_info.appendChild(vote_area);

        httpGetAsync("/user_status", function(text){
            var userstatus = JSON.parse(text);
            console.log(userstatus);
            if (userstatus["logined"] == true) {
                var plus_b = document.createElement("div");
                plus_b.className = "vote-up";
                plus_b.onclick = function() { httpGetAsync("/vote_image?vote=true&id=" + id, function(res){
                    if (parseInt(res) !== NaN) {
                        score.textContent = "Score: " + res;
                    } else {
                        score.textContent = res;
                    }
                })};
                vote_up_a.appendChild(plus_b);

                var minus_b = document.createElement("div");
                minus_b.className = "vote-down";
                minus_b.onclick = function() { httpGetAsync("/vote_image?vote=false&id=" + id, function(res){
                    if (parseInt(res) !== NaN) {
                        score.textContent = "Score: " + res;
                    } else {
                        score.textContent = res;
                    }
                })};

                vote_down_a.appendChild(minus_b);

                vote_area.appendChild(vote_up_a);
                vote_area.appendChild(vote_down_a);

                if (userstatus["name"] == body["uploader"]) {
                    var l = document.createElement("a");
                    l.href = "/delete/" + id;
                    l.textContent = "Delete image";
                    image_info.appendChild(l);
                    image_info.appendChild(document.createElement("br"));
                }
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
