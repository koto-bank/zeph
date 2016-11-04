var LOAD_AT_A_TIME = 25;
var TAGS_SET = false;

function httpGetAsync(theUrl, callback) {
    var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4 && xmlHttp.status == 200) { callback(xmlHttp.responseText); }
    }
    xmlHttp.open("GET", theUrl, true);
    xmlHttp.send(null);
}

function getUrlParameter(name) {
    name = name.replace(/[\[]/, '\\[').replace(/[\]]/, '\\]');
    var regex = new RegExp('[\\?&]' + name + '=([^&#]*)');
    var results = regex.exec(location.search);
    return results === null ? '' : decodeURIComponent(results[1].replace(/\+/g, ' '));
};

function loadMore() {
    var image_block = document.getElementById("images");
    let count = image_block.children.length;
    var query = "/more?offset="+count;

    if (window.location.pathname.startsWith("/search")) {
        query = query + "&q=" + getUrlParameter("q");
    }

    httpGetAsync(query, function(text){
        var body = JSON.parse(text);
        if (body.length < LOAD_AT_A_TIME) {
            document.getElementById("more-button").parentNode.removeChild(document.getElementById("more-button"))
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
            image_block.appendChild(link);
        });

        if (!TAGS_SET) {
            var imgs = Array.from(document.getElementsByClassName("thumbnail"));
            var tags_block = document.getElementById("tags");
            var tags = new Set(imgs.reduce(function(arr, im) {
                arr.push(im.title.split(" ")[0]);
                return arr
            },[]));

            tags.forEach(function(tag) {
                var link = document.createElement("a");
                link.textContent = tag;
                link.href = "/search?q=" + tag;
                tags_block.appendChild(link);
                tags_block.appendChild(document.createElement("br"));
            });
            TAGS_SET = true
        }
    });
}

function drawUploadOrLogin() {
    var main_form = document.getElementById("login-or-upload-form");

    httpGetAsync("/user_status", function(text){
        var body = JSON.parse(text);

        if (body["logined"] == true) {
            true
        } else {
            var form = document.createElement("form");
                form.action="/upload_image";
                form.method="POST";
                form.enctype="multipart/form-data";
            var file = document.createElement("input")
                file.type="file";
                file.name="image";
                file.accept="image/*";
            var tags = document.createElement("input")
                tags.type="text";
                tags.name="tags";
                tags.placeholder="Space separated tags";
            var sbm = document.createElement("input");
                sbm.type="submit";
                sbm.value="Upload";
            form.appendChild(file);
            form.appendChild(tags);
            form.appendChild(sbm);
            main_form.appendChild(form);
        }
    });
}

function showUploadOrLogin() {
    var form = document.getElementById("login-or-upload-form");

    if (form.style.bottom != "7%") {
        form.style.bottom = "7%"
    } else {
        form.style.bottom = "-100%"
    }
}

window.onload = function() {
    loadMore();
    drawUploadOrLogin();

    document.getElementById("tag-search-field").value = getUrlParameter("q");
}
