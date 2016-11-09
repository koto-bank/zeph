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

    var vote_up_a = document.createElement("a");
    vote_up_a.href = "#";
    vote_up_a.style.display = "inline-block";
    var vote_down_a = document.createElement("a");
    vote_down_a.href = "#";
    vote_down_a.style.display = "inline-block";

    httpGetAsync("/user_status", function(text){

        var uploader = document.getElementById("uploader").value;
        var score = document.getElementById("score");

        var userstatus = JSON.parse(text);
        console.log(userstatus);
        if (userstatus["logined"] == true && userstatus["name"] == uploader) {
            var l = document.createElement("a");
            l.href = "/delete/" + id;
            l.textContent = "Delete image";
            image_info.appendChild(l);
            image_info.appendChild(document.createElement("br"));
        }

        if (userstatus["logined"] == true && uploader != "sync") {
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

            var vote_area = document.getElementById("vote-area");

            vote_area.appendChild(vote_up_a);
            vote_area.appendChild(vote_down_a);
        }
    });
}

window.onload = function() {
    load();
}
