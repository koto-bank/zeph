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

function setAttrs(elem, attrs) {
    for (var a in attrs) {
        elem.setAttribute(a, attrs[a]);
    }
}

function loadMore() {
    var image_block = document.getElementById("images");;
    var query = "/more?offset="+image_block.children.length;

    if (window.location.pathname.startsWith("/search")) {
        query = query + "&q=" + getUrlParameter("q");
    }

    httpGetAsync(query, function(text){
        var body = JSON.parse(text);
        if (body.length < LOAD_AT_A_TIME) {
            document.getElementById("more-button").parentNode.removeChild(document.getElementById("more-button"));
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
                return arr;
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
    var upload_button = document.getElementById("upload-button");

    httpGetAsync("/user_status", function(text){
        var body = JSON.parse(text);

        if (body["logined"] == false) {
            var form = document.createElement("form");
            setAttrs(form, {
                action: "/login",
                method: "POST"
            });

            var login = document.createElement("input");
            setAttrs(login,{
                type: "text",
                name: "login",
                placeholder: "Login"
            });
            var pass = document.createElement("input");
            setAttrs(pass, {
                type: "password",
                name: "password",
                placeholder: "Password"
            });
            var confirm_pass = document.createElement("input");
            setAttrs(confirm_pass, {
                type: "password",
                name: "confirm_password",
                placeholder: "Confirm password"
            });
            var confirm_pass_br = document.createElement("br");

            var sbm = document.createElement("input");
            setAttrs(sbm, {
                type: "submit",
                value: "Login"
            });

            var lor = document.createElement("button");
            lor.textContent = "Sign up";
            lor.onclick = function() {
                if (lor.textContent == "Sign up") {
                    lor.textContent = "Sign in";
                    sbm.value = "Register";
                    login.placeholder = "New login";
                    pass.placeholder = "New password";

                    form.action = "/adduser";

                    form.insertBefore(confirm_pass, sbm);
                    form.insertBefore(confirm_pass_br, sbm);
                } else {
                    lor.textContent = "Sign up";
                    sbm.value = "Login";
                    login.placeholder = "Login";
                    pass.placeholder = "Password";

                    form.action = "/login";

                    try {
                        form.removeChild(confirm_pass);
                        form.removeChild(confirm_pass_br);
                    } catch(err) { }
                }
            }

            main_form.appendChild(lor);
            form.appendChild(login);
            form.appendChild(pass);



            form.appendChild(sbm);
            main_form.appendChild(form);
        } else {
            upload_button.textContent = "Upload image (as " + body["name"] + ")";
            var form = document.createElement("form");
            setAttrs(form, {
                action: "/upload_image",
                method: "POST",
                enctype: "multipart/form-data"
            });
            var file = document.createElement("input");
            setAttrs(file, {
                type: "file",
                name: "image",
                accept: "image/*"});
            var tags = document.createElement("input");
            setAttrs(tags,{
                type: "text",
                name: "tags",
                placeholder: "Space separated tags"});
            var sbm = document.createElement("input");
            setAttrs(sbm,{
                type: "submit",
                value: "Upload"});
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
        form.style.bottom = "7%";
    } else {
        form.style.bottom = "-100%";
    }
}

window.onload = function() {
    loadMore();
    drawUploadOrLogin();

    document.getElementById("tag-search-field").value = getUrlParameter("q");
}
