async function fetch_torrent(id) {
    try {
        const response = await fetch(`/api/torrent?id=${id}`);
        if (!response.ok) {
            throw new Error(`Response status: ${response.status}`);
        }
        const json = await response.json();
        write_info(json);
    } catch (error) {
        console.log(error.message);
    }
}

async function fetch_comments(id) {
    try {
        const response = await fetch(`/api/comments?id=${id}`);
        if (!response.ok) {
            throw new Error(`Response status: ${response.status}`);
        }
        const json = await response.json();
        write_comments(json);
    } catch (error) {
        console.log(error.message);
    }
}

function write_comments(comments) {
    let mainPanel = document.querySelector("#collapse-comments");
    for (let i = 0; i < comments.length; i++) {
        let comment = comments[i];
        let element = document.createElement("div");
        element.classList.add("panel", "panel-default", "comment-panel");
        element.setAttribute("id", `com-${i + 1}`);
        mainPanel.appendChild(element);
        let body = document.createElement("div");
        body.classList.add("panel-body");
        element.appendChild(body);
        let profile = document.createElement("div");
        profile.classList.add("col-md-2");
        body.appendChild(profile);
        let userParent = document.createElement("p");
        profile.appendChild(userParent);
        let user = document.createElement("a");
        user.classList.add("text-default");
        user.href = `/user/${comment.submitter}`;
        user.setAttribute("data-toggle", "tooltip");
        user.setAttribute("title", "User");
        user.textContent = comment.submitter;
        userParent.appendChild(user);
        let picture = document.createElement("img");
        picture.classList.add("avatar");
        picture.src = (comment.default_pfp) ? "/default.png" : `/pfps/${comment.submitter}.png`;
        picture.alt = "User";
        profile.appendChild(picture);
        let commentParent = document.createElement("div");
        commentParent.classList.add("col-md-10", "comment");
        body.appendChild(commentParent);
        let commentDetails = document.createElement("div");
        commentDetails.classList.add("row", "comment-details");
        commentParent.appendChild(commentDetails);
        let commentLink = document.createElement("a");
        commentLink.href = `#com-${i + 1}`;
        commentDetails.appendChild(commentLink);
        let commentDate = document.createElement("small");
        commentDate.setAttribute("data-timestamp-swap", "");
        commentDate.setAttribute("data-timestamp", comment.date);
        commentDate.setAttribute("title", formatTimestamp(comment.date_created));
        commentDate.textContent = timeSince(comment.date_created);
        commentLink.appendChild(commentDate);
        let commentActions = document.createElement("div");
        commentActions.classList.add("comment-actions");
        commentDetails.appendChild(commentActions);
        let commentBody = document.createElement("div");
        commentBody.classList.add("row", "comment-body");
        commentParent.appendChild(commentBody);
        let commentContents = document.createElement("div");
        commentContents.setAttribute("id", `torrent-comment${comment.id}`);
        commentContents.classList.add("comment-content");
        commentContents.setAttribute("markdown-text", "");
        commentContents.textContent = comment.text;
        commentBody.appendChild(commentContents);
    }
    trigger_markdown();
}

function write_info(torrent) {
    let panel = document.querySelector(".container>.panel:nth-child(1)");
    if (!torrent.trusted && !torrent.remake) {
        panel.classList.add("panel-default");
    } else if (torrent.trusted) {
        panel.classList.add("panel-success");
    } else if (torrent.remake) {
        panel.classList.add("panel-danger");
    }
    document.querySelector("#title").textContent = torrent.title;
    document.head.querySelector("title").textContent = `${torrent.title} :: Nyaa`;
    let category_broad = Number(`${torrent.category.toString()[0]}0`);
    let categories = document.querySelectorAll("#category>a");
    categories[0].href = `${location.origin}/?category=${category_broad}`;
    categories[0].textContent = get_category(category_broad)[0];
    categories[1].href = `${location.origin}/?category=${torrent.category}`;
    categories[1].textContent = get_category(torrent.category)[2].textContent.replace("- ", "");
    document.querySelector("#date").setAttribute("data-timestamp", torrent.date);
    document.querySelector("#date").setAttribute("title", timeSince(torrent.date));
    document.querySelector("#date").textContent = formatTimestamp(torrent.date);
    let submitterParent = document.querySelector("#submitter").parentElement;
    if (torrent.anonymous) {
        document.querySelector("#submitter").remove();
        submitterParent.textContent = "Anonymous";
    } else {
        document.querySelector("#submitter").href = `${location.origin}/user/${torrent.submitter}`;
        document.querySelector("#submitter").textContent = torrent.submitter;
    } 
    document.querySelector("#seeders").textContent = torrent.seeders;
    document.querySelector("#information").textContent = torrent.information;
    document.querySelector("#leechers").textContent = torrent.leechers;
    document.querySelector("#size").textContent = torrent.size;
    document.querySelector("#completed").textContent = torrent.completed;
    document.querySelector("#info-hash>kbd").textContent = torrent.info_hash;
    document.querySelector("#torrent").href = `${location.origin}${torrent.torrent}`;
    document.querySelector("#magnet").href = torrent.magnet;
    let description = (torrent.partial) ? "#### This torrent has not been scraped! Check again later." : torrent.description;
    document.querySelector("#torrent-description").textContent = description;
    trigger_markdown();
    let torrent_root = document.querySelector(".torrent-file-list");
    if (torrent.files.length == 0) {
        createPath(torrent_root, torrent.name, false, torrent.size);
    } else {
        let rootFolder = createPath(torrent_root, torrent.name, true);
        for (let i = 0; i < torrent.files.length; i++) {
            let file = torrent.files[i];
            let selector = (file.parts.length > 0) ? "" : ".torrent-file-list>ul>li";
            for (let i = 0; i < file.parts.length; i++) {
                let folder = file.parts[i];
                let parent = (selector != "") ? rootFolder.querySelector(selector) : rootFolder;
                selector += `ul>li[title="${folder}"]`;
                createPath(parent, folder, true);
            }
            let folder = document.querySelector(selector);
            createPath(folder, file.path, false, file.length);
        }
    }
    document.querySelector("#comments>.panel-heading>a>h3").textContent = `Comments - ${torrent.comments}`;
}

function createPath(parent, name, folder = false, size = "") {
    let upper = parent.querySelector("ul") || document.createElement("ul");
    parent.appendChild(upper);
    let middle = document.createElement("li");
    middle.setAttribute("title", name);
    upper.appendChild(middle);
    if (folder) {
        let target = document.createElement("a");
        target.classList.add("folder");
        target.setAttribute("role", "button");
        target.append(name);
        middle.appendChild(target);
        let inner = document.createElement("i");
        inner.classList.add("fa", "fa-folder-open");
        target.prepend(inner);
    } else {
        let target = document.createElement("i");
        target.classList.add("fa", "fa-file");
        middle.appendChild(target);
        middle.append(`${name} `);
        let span = document.createElement("span");
        span.classList.add("file-size");
        span.textContent = `(${size})`;
        middle.appendChild(span);
    }
    if (parent.parentElement.parentElement == document.querySelector(".torrent-file-list")) {
        upper.setAttribute("data-show", "yes");
    } else if (parent != document.querySelector(".torrent-file-list")) {
        upper.style = "display: block;";
    }
    return (folder) ? middle : upper;
}

function trigger_markdown() {
    let markdownTargets = document.querySelectorAll('[markdown-text],[markdown-text-inline]');
    for (var i = 0; i < markdownTargets.length; i++) {
        let target = markdownTargets[i];
        let rendered;
        let markdownSource = target.textContent;
        if (target.attributes['markdown-no-images']) {
            markdown.disable('image');
        } else {
            markdown.enable('image');
        }
        if (target.attributes['markdown-text-inline']) {
            rendered = markdown.renderInline(markdownSource);
        } else {
            rendered = markdown.render(markdownSource);
        }
        target.innerHTML = rendered;
    }
}

function get_category(id) {
    let id_as_str = id.toString();
    let id_str = `${id_as_str[0]}_${id_as_str[1]}`;
    let option = document.querySelector(`#navFilter-category>.btn-group>select>option[value="${id_str}"]`);
    return [option.getAttribute("title"), id_str, option];
}

window.addEventListener("load", function () {
    let id = Number(location.pathname.replace("/view/", ""));
    fetch_torrent(id);
    fetch_comments(id);
});
