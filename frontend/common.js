function click_dropdown(event) {
    let dropdown = event.target.closest(".btn-group,.dropdown");
    dropdown.classList.toggle("open");
    let dropdowns = document.querySelectorAll(".btn-group,.dropdown");
    for (let i = 0; i < dropdowns.length; i++) {
        if (dropdowns[i] !== dropdown) {
            dropdowns[i].classList.remove("open");
        }
    }
}

function click_dropdown_child(event) {
    let button = event.target.closest("li");
    button.classList.add("selected");
    let [title, option, options] = button_title(button);
    if (option != null) {
        option.setAttribute("selected", "selected");
        for (let i = 0; i < options.length; i++) {
            if (options[i] != option) {
                options[i].removeAttribute("selected");
            }
        }
    }
    let dropdown_button = button.closest(".btn-group,.dropdown").querySelector(".dropdown-toggle");
    dropdown_button.setAttribute("title", title);
    dropdown_button.querySelector(".filter-option").textContent = title;
    let buttons = button.parentElement.children;
    for (let i = 0; i < buttons.length; i++) {
        if (buttons[i] != button) {
            buttons[i].classList.remove("selected");
        }
    }
}

function button_title(button) {
    let contents = button.querySelector("a>span").textContent;
    let options = button.closest(".btn-group").querySelector("select")
    let match = null;
    for (let i = 0; i < options.length; i++) {
        if (options[i].textContent == contents) {
            match = options[i];
            break;
        }
    }
    contents = (match == null) ? contents : match.getAttribute("title");
    return [contents, match, (match == null) ? null : match.parentElement];
}

function get_category(id) {
    let id_as_str = id.toString();
    let id_str = `${id_as_str[0]}_${id_as_str[1]}`;
    let option = document.querySelector(`#navFilter-category>.btn-group>select>option[value="${id_str}"]`);
    return [option.getAttribute("title"), id_str];
}

function getQueryOptions() {
    let searchBar = document.querySelector(".search-bar");
    let query = searchBar.value;
    let filter = document.querySelector("#navFilter-criteria>div>button").getAttribute("title");
    let category = Number(document.querySelector("#navFilter-category>div>select>option[selected]").getAttribute("value").replace("_", ""));
    let newFilter;
    if (filter == "No filter") {
        newFilter = "None";
    } else if (filter == "No remakes") {
        newFilter = "NoRemakes";
    } else {
        newFilter = "Trusted";
    }
    let result = (query != '') ? `query=${encodeURIComponent(query)}` : '';
    result = (newFilter != "None") ? (result != '') ? `${result}&filter=${newFilter}` : `filter=${newFilter}` : result;
    result = (category != 0) ? (result != '') ? `${result}&category=${category}` : `category=${category}` : result;
    return result;
}

function init_markdown() {
    if (window.markdownit === undefined) {
        return;
    }
    var markdownOptions = {
        html: false,
        breaks: true,
        linkify: true,
        typographer: true
    }
    window.markdown = window.markdownit(markdownOptions);
    markdown.renderer.rules.table_open = function (tokens, idx) {
        return '<table class="table table-striped table-bordered" style="width: auto;">';
    }
    var defaultRender = markdown.renderer.rules.link_open ||
    function (tokens, idx, options, env, self) {
        return self.renderToken(tokens, idx, options);
    };
    markdown.renderer.rules.link_open = function (tokens, idx, options, env, self) {
        tokens[idx].attrPush(['rel',
        'noopener nofollow noreferrer']);
        return defaultRender(tokens, idx, options, env, self);
    }
    const defaultImageRender = markdown.renderer.rules.image ||
    function (tokens, idx, options, env, self) {
        return self.renderToken(tokens, idx, options);
    };
    markdown.renderer.rules.image = function (tokens, idx, options, env, self) {
        function getPhotonURL(inURL) {
            let hash = 0;
            for (let i = 0; i < inURL.length; i++) {
                const char = inURL.charCodeAt(i);
                hash = (hash << 5) - hash + char;
                hash &= hash;
            }
            const urlWhitelist = [
            ];
            var photonURL;
            if (typeof (window.URL) != 'function') {
                photonURL = inURL;
            } else {
                let urlObj = new URL(inURL, location.href);
                var urlHost = urlObj.hostname.split('.').slice( - 2).join('.');
                if (urlWhitelist.includes(urlHost) || urlObj.protocol == 'data:') {
                    photonURL = inURL;
                } else if (
                    urlObj.username ||
                    urlObj.password ||
                    urlObj.port ||
                    (urlObj.search && !urlObj.search.match(/^\?\d*$/))
                ) {
                    photonURL = `http://imageproxy.i2p/?url=${encodeURIComponent(inURL)}&l=9`;
                } else {
                    photonURL = `http://imageproxy.i2p/?url=${urlObj.host}${urlObj.pathname}&l=9`;
                }
            }
            return photonURL;
        }
        let token = tokens[idx];
        let aIndex = token.attrIndex('src');
        const imageURL = (aIndex < 0) ? null : token.attrs[aIndex][1];
        if (window.markdown_proxy_images && imageURL) {
            token.attrs[aIndex][1] = getPhotonURL(imageURL);
        }
        return defaultImageRender(tokens, idx, options, env, self);
    }
}

function init_markdown_editors() {
    if (window.markdown === undefined) {
        return;
    }
    var markdownEditors = Array.prototype.slice.call(document.querySelectorAll('.markdown-editor'));
    markdownEditors.forEach(
        function (markdownEditor) {
            var fieldName = markdownEditor.getAttribute('data-field-name');
            var previewTabSelector = '#' + fieldName + '-preview-tab';
            var targetSelector = '#' + fieldName + '-markdown-target';
            var sourceSelector = markdownEditor.querySelector('.markdown-source');
            var previewTabEl = markdownEditor.querySelector(previewTabSelector);
            var targetEl = markdownEditor.querySelector(targetSelector);
            previewTabEl.addEventListener(
                'click',
                function () {
                    var rendered = markdown.render(sourceSelector.value.trim());
                    targetEl.innerHTML = rendered;
                }
            );
        }
    );
}

window.onclick = function (event) {
    if (!event.target.matches(".dropdown-toggle") && !event.target.closest(".dropdown-toggle")) {
        let dropdowns = document.querySelectorAll(".btn-group,.dropdown ");
        for (let i = 0; i < dropdowns.length; i++) {
            let dropdown = dropdowns[i];
            if (dropdown.classList.contains("open")) {
                dropdown.classList.remove("open");
            }
        }
    }
}

window.onload = function () {
    if ("dark" === localStorage.getItem("theme")) {
        document.body.classList.add("dark");
    }
    let dropdowns = document.querySelectorAll(".btn-group,.dropdown");
    for (let i = 0; i < dropdowns.length; i++) {
        let dropdown = dropdowns[i].querySelector(".dropdown-toggle");
        dropdown.onclick = click_dropdown;
        let dropdown_buttons = dropdowns[i].querySelectorAll(".dropdown-menu>ul>li");
        for (let i = 0; i < dropdown_buttons.length; i++) {
            dropdown_buttons[i].onclick = click_dropdown_child;
        }
    }
    let searchButton = document.querySelector(".search-btn>button");
    searchButton.onclick = function () {
        let queryOptions = getQueryOptions();
        window.location.href = `${location.origin}/${(queryOptions != "") ? `?${queryOptions}` : ""}`
        return false;
    }
}

window.addEventListener("load", init_markdown);
window.addEventListener("load", init_markdown_editors);

window.formatTimestamp = function (timestamp) {
    let date = new Date(timestamp * 1000);
    const datevalues = [
        date.getFullYear(),
        date.getMonth() + 1,
        date.getDate(),
        date.getHours(),
        date.getMinutes(),
        date.getSeconds(),
    ];
    const seconds = (datevalues[4] < 10) ? `0${datevalues[4]}` : datevalues[4].toString();

    return `${datevalues[0]}-${datevalues[1]}-${datevalues[2]} ${datevalues[3]}:${seconds}`;
}

window.timeSince = function (date) {
    let timestamp = Math.floor(new Date().getTime() / 1000 - date);
    let output = "";

    function length(output) {
        return output.split(" ").length >= 6 + 1;
    }

    let years = Math.floor(timestamp / 31536000);
    output = output + ((years <= 0 || length(output)) ? "" : years + ((years > 1) ? " years " : " year "));
    timestamp -= years * 31536000;
    let months = Math.floor(timestamp / 2592000);
    output = output + ((months <= 0 || length(output)) ? "" : months + ((months > 1) ? " months " : " month "));
    timestamp -= months * 2592000;
    let weeks = Math.floor(timestamp / 604800);
    output = output + ((weeks <= 0 || length(output)) ? "" : weeks + ((weeks > 1) ? " weeks " : " week "));
    timestamp -= weeks * 604800;
    let days = Math.floor(timestamp / 86400);
    output = output + ((days <= 0 || length(output)) ? "" : days + ((days > 1) ? " days " : " day "));
    timestamp -= days * 86400;
    let hours = Math.floor(timestamp / 3600);
    output = output + ((hours <= 0 || length(output)) ? "" : hours + ((hours > 1) ? " hours " : " hour "));
    timestamp -= hours * 3600;
    let minutes = Math.floor(timestamp / 60);
    output = output + ((minutes <= 0 || length(output)) ? "" : minutes + ((minutes > 1) ? " minutes " : " minute "));
    timestamp -= minutes * 60;
    let seconds = Math.floor(timestamp);
    output = output + ((seconds <= 0 || length(output)) ? "" : seconds + ((seconds > 1) ? " seconds " : " second "));

    return output + "ago";
}
