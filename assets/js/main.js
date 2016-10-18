var LOAD_AT_A_TIME = 25;

function httpGetAsync(theUrl, callback)
{
    var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4 && xmlHttp.status == 200)
            callback(xmlHttp.responseText);
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
            link.href = "/show/"+image.link;
            var im = document.createElement("div");
            im.title = image.tags.join(" ");
            im.className = "thumbnail";
            im.style.backgroundImage = "url(/images/"+image.name+")";

            link.appendChild(im);
            image_block.appendChild(link);
        });
    });
}

function showUploadForm() {
    var form = document.getElementById("upload-form");

    if (form.style.bottom != "7%") {
        form.style.bottom = "7%"
    } else {
        form.style.bottom = "-100%"
    }
}

window.onload = function() {
    loadMore();

    document.getElementById("tag-search-field").value = getUrlParameter("q");
}
