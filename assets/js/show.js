var LOAD_AT_A_TIME = 25; // Сколько картинок за раз грузить
var DONE_LOADING = false; // Все ли картинки загружены
var LOADING_IN_PROGRESS = false; // Грузится ли сейчас картинки

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

        var uploader = document.getElementById("uploader").getAttribute("value");
        var score = document.getElementById("score");
        var image_info =document.getElementById("image-info");

        var userstatus = JSON.parse(text);
        if (userstatus["logined"] == true && userstatus["name"] == uploader) {
            var l = document.createElement("a");
            l.href = "/delete/" + id;
            l.textContent = "Delete image";

            image_info.insertBefore(l, image_info.firstChild);
            image_info.insertBefore(document.createElement("br"), l.nextSibling);
        }

        if (userstatus["logined"] == true && uploader !== "sync") {
            var plus_b = document.createElement("div");
            plus_b.className = "vote-up";
            plus_b.onclick = function() { httpGetAsync("/vote_image?vote=true&id=" + id, function(res){
                console.log(res);
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

function loadSimiliar() {
    LOADING_IN_PROGRESS = true;
    var reg = /show\/(\d+)/;
    var id = reg.exec(window.location.pathname)[1];
    
    var similiar_block = document.getElementById("similiar");;
    var query = "/similiar?id=" + id + "&offset="+similiar_block.children.length;

    httpGetAsync(query, function(text){
        var body = JSON.parse(text);
        if (body.length < LOAD_AT_A_TIME) {
            DONE_LOADING = true;
        }

        body.forEach(function(image) {
            var link = document.createElement("a");
            link.href = "/show/"+image.id;
            link.target = "_blank";
            var im = document.createElement("div");
            im.title = image.tags.join(" ");
            im.className = "thumbnail";
            im.style.backgroundImage = "url(\"/images/preview/"+image.name+"\")";

            link.appendChild(im);
            similiar_block.appendChild(link);
        });
        LOADING_IN_PROGRESS = false;
    });
}

window.onload = function() {
    load();
    loadSimiliar();
}

window.onscroll = function(ev) {
    if ((window.innerHeight + window.scrollY) >= document.body.offsetHeight) {
        if (!DONE_LOADING && !LOADING_IN_PROGRESS){
            loadSimiliar();
        }
    }
};
