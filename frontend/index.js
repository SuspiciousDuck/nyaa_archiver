async function fetch_torrents(page, options = "") {
    try {
        const response = await fetch(`/api/?page=${page - 1}${options}`);
        if (!response.ok) {
            throw new Error(`Response status: ${response.status}`);
        }
        const results = await response.json();
        for (let i = 0; i < results.length; i++) {
            add_torrent(results[i]);
        }
        if (results.length == 0) {
            no_pages_found();
        } else {
            no_pages_found(true);
        }
    } catch (error) {
        console.error(error.message);
    }
}

async function fetch_pages(options = "") {
    try {
        const response = await fetch(`/api/pages${options}`);
        if (!response.ok) {
            throw new Error(`Response status: ${response.status}`);
        }
        const json = await response.json();
        return [Number(json.pages), Number(json.results)];
    } catch (error) {
        console.error(error.message);
    }
}

function update_page(page, options = "") {
    let queryOptions = getQueryOptions();
    let table = document.querySelector(".table>tbody");
    table.querySelectorAll("tr").forEach(element => {
        element.remove();
    });
    if (options == "" && queryOptions !== "") {
        options = `&${queryOptions}`;
    } else if (!options.includes("query") && !options.includes("filter") && !options.includes("category") && queryOptions !== "") {
        options = `${options}&${queryOptions}`;
    }
    fetch_torrents(page, options);
}

function update_page_attributes(newPage) {
    let pagination = document.querySelector(".pagination");
    if (pagination.querySelectorAll("li:not([rel])").length == 0) {
        let prevButton = pagination.querySelector("li[rel=\"prev\"]");
        let nextButton = pagination.querySelector("li[rel=\"next\"]");
        prevButton.classList.add("disabled");
        nextButton.classList.add("disabled");
        console.error("No pages found!");
        document.querySelector("body>div.container>div.center>nav").hidden = true;
        return;
    } else {
        document.querySelector("body>div.container>div.center>nav").hidden = false;
    }
    let newButton = pagination.querySelector(`li[page="${newPage}"]`);
    newButton.classList.add("active");
    if (!newButton.querySelector("span")) {
        let newSpan = document.createElement("span");
        newSpan.classList.add("sr-only");
        newSpan.textContent = "(current)";
        newButton.querySelector("a").appendChild(newSpan);
    }
    let prevButton = pagination.querySelector("li[rel=\"prev\"]");
    let nextButton = pagination.querySelector("li[rel=\"next\"]");
    if (newPage - 1 <= 0) {
        prevButton.classList.add("disabled");
    } else {
        prevButton.classList.remove("disabled");
    }
    let allButtons = pagination.querySelectorAll("li:not([rel])");
    if (newPage + 1 > allButtons.length) {
        nextButton.classList.add("disabled");
    } else {
        nextButton.classList.remove("disabled");
    }
    for (let i = 0; i < allButtons.length; i++) {
        let button = allButtons[i];
        if (button == newButton) {
            continue
        }
        button.classList.remove("active");
        let span = button.querySelector("span");
        if (span != null) {
            span.remove();
        }
    }
}

function createPage(page, maxPage, skip, selected) {
    let exists = document.querySelector(`.pagination>li[page="${page}"]`);
    let next = document.querySelector(".pagination>li[rel=\"next\"]");
    if (page > maxPage || page < 1 || exists !== null && !exists.getAttribute("active") == selected) {
        return;
    }
    if (exists !== null && selected && !skip) {
        exists.classList.add("active");
        let newSpan = document.createElement("span");
        newSpan.classList.add("sr-only");
        newSpan.textContent = "(current)";
        exists.querySelector("a").appendChild(newSpan);
        return;
    } else if (exists !== null && !selected && !skip) {
        exists.classList.remove("active");
        if (!!exists.querySelector("span")) {
            exists.querySelector("span").remove();
        }
        return;
    }
    let pagination = document.querySelector(".pagination");
    let newButton = document.createElement("li");
    if (skip) {
        newButton.classList.add("disabled");
        let newA = document.createElement("a");
        newA.setAttribute("role", "button");
        newA.textContent = "...";
        newButton.appendChild(newA);
        pagination.insertBefore(newButton, next);
        return;
    }
    newButton.setAttribute("page", page);
    if (selected) {
        newButton.classList.add("active");
    }
    pagination.insertBefore(newButton, next);
    let newA = document.createElement("a");
    newA.setAttribute("role", "button");
    newA.textContent = page;
    newButton.appendChild(newA);
    if (selected) {
        let newSpan = document.createElement("span");
        newSpan.classList.add("sr-only");
        newSpan.textContent = "(current)";
        newA.appendChild(newSpan);
    }
}

function no_pages_found(remove = false) {
    let container = document.querySelector("body>div.container");
    let table = container.querySelector(".table-responsive");
    if (remove) {
        table.hidden = false;
        if (container.querySelector("h3") !== null) {
            container.querySelector("h3").remove();
        }
        return;
    }
    if (container.querySelector("h3") !== null) {
        return;
    }
    table.hidden = true;
    let header = document.createElement("h3");
    header.textContent = "No results found";
    container.insertBefore(header, container.querySelector("div.center"));
}

async function update_page_buttons(newPage, options = "") {
    // always show pages 1 and 2
    // always show previous 6 pages
    // always show next 5 pages
    // max pages is 14, if higher then replace range [3..current page - 7] with ... (both inclusive)
    let [maxPage, results] = await fetch_pages(options);
    let pagination = document.querySelector(".pagination");
    let oldPages = pagination.querySelectorAll("li:not([rel])");
    oldPages.forEach(e => e.remove());
    createPage(1, maxPage, false, (1 == newPage));
    createPage(2, maxPage, false, (2 == newPage));
    if (2 + 6 + 1 < newPage) {
        createPage(3, maxPage, true, false); // ... button
    }
    for (let i = 6; i > 0; i--) {
        createPage(newPage - i, maxPage, false, false);
    }
    createPage(newPage, maxPage, false, true);
    for (let i = 1; i < 6; i++) {
        createPage(newPage + i, maxPage, false, false);
    }
    update_page_attributes(newPage);
    apply_page_button_functions();
    if (options != "") {
        let info = document.querySelector(".pagination-page-info");
        if (info === null) {
            info = document.createElement("div");
            info.classList.add("pagination-page-info");
        }
        if (maxPage >= 1) {
            let lhs = (newPage - 1) * 75 + 1;
            let total = (results >= 75) ? 75 : results;
            info.textContent = `Displaying results ${lhs}-${lhs + total - 1} out of ${results} results.\nPlease refine your search results if you can't find what you were looking for.`
        } else {
            info.textContent = "Displaying results 0-0 out of 0 results.\nPlease refine your search results if you can't find what you were looking for.";
        }
        document.querySelector("body>div.container>div.center").appendChild(info);
    }
}

function apply_page_button_functions() {
    let pageButtons = document.querySelectorAll(".pagination>li");
    for (let i = 0; i < pageButtons.length; i++) {
        let attr = pageButtons[i].closest("li").getAttribute("rel"); 
        if (attr == "prev") {
            pageButtons[i].onclick = function (event) {
                let disabled = event.target.closest("li").classList.contains("disabled");
                if (disabled) {
                    return;
                }
                let page_num = get_current_page() - 1;
                let queryOptions = getQueryOptions();
                update_page_buttons(page_num, `?${queryOptions}`);
                //update_page_attributes(page_num);
                update_page(page_num);
            };
            continue;
        }
        if (attr == "next") {
            pageButtons[i].onclick = function (event) {
                let disabled = event.target.closest("li").classList.contains("disabled");
                if (disabled) {
                    return;
                }
                let page_num = get_current_page() + 1;
                let queryOptions = getQueryOptions();
                update_page_buttons(page_num, `?${queryOptions}`);
                //update_page_attributes(page_num);
                update_page(page_num);
            };
            continue;
        }
        if (pageButtons[i].getAttribute("page") === null) {
            continue;
        }
        pageButtons[i].onclick = function () {
            set_page(pageButtons[i].closest("li"));
        };
    }
}

function get_current_page() {
    return Number(document.querySelector(".pagination>li.active").getAttribute("page"));
}

function set_page(elem) {
    let button = elem.querySelector("a");
    let page_num = Number(button.textContent.replace("(current)", ""));
    let queryOptions = getQueryOptions();
    update_page_buttons(page_num, `?${queryOptions}`);
    update_page(page_num);
}

function get_category(id) {
    let id_as_str = id.toString();
    let id_str = `${id_as_str[0]}_${id_as_str[1]}`;
    let option = document.querySelector(`#navFilter-category>.btn-group>select>option[value="${id_str}"]`);
    return [option.getAttribute("title"), id_str];
}

function add_torrent(obj) {
    let table = document.querySelector(".table>tbody");
    // Row
    let newElement = document.createElement("tr");
    if (!obj.trusted && !obj.remake) {
        newElement.classList.add("default");
    } else if (obj.trusted) {
        newElement.classList.add("success");
    } else if (obj.remake) {
        newElement.classList.add("danger");
    }
    table.appendChild(newElement);
    // Category
    let categoryParent = document.createElement("td");
    newElement.appendChild(categoryParent);
    let category = document.createElement("a");
    let [cat, cat_id] = get_category(obj.category);
    category.href = `?category=${obj.category}`;
    category.title = cat;
    categoryParent.appendChild(category);
    let categoryImg = document.createElement("img");
    categoryImg.src = cat_id + ".png";
    categoryImg.alt = cat;
    categoryImg.classList.add("category-icon");
    category.appendChild(categoryImg);
    // Title
    let titleParent = document.createElement("td");
    titleParent.colSpan = 2;
    newElement.appendChild(titleParent);
    if (obj.comments > 0) {
        let comments = document.createElement("a");
        comments.classList.add("comments");
        comments.href = `/view/${obj.id}#comments`;
        comments.title = (obj.comments > 1) ? `${obj.comments} comments` : `1 comment`;
        comments.textContent = `${obj.comments}`;
        titleParent.appendChild(comments);
        let commentsIcon = document.createElement("i");
        commentsIcon.classList.add("fa", "fa-comments-o");
        comments.prepend(commentsIcon);
    }
    let title = document.createElement("a");
    title.href = `/view/${obj.id}`;
    title.title = obj.title;
    title.textContent = obj.title;
    titleParent.appendChild(title);
    // Links
    let linksParent = document.createElement("td");
    linksParent.classList.add("text-center");
    newElement.appendChild(linksParent);
    let torrentLink = document.createElement("a");
    torrentLink.href = location.origin + obj.torrent;
    linksParent.appendChild(torrentLink);
    let torrentIcon = document.createElement("i");
    torrentIcon.classList.add("fa", "fa-fw", "fa-download");
    torrentLink.appendChild(torrentIcon);
    let magnetLink = document.createElement("a");
    magnetLink.href = obj.magnet;
    linksParent.appendChild(magnetLink);
    let magnetIcon = document.createElement("i");
    magnetIcon.classList.add("fa", "fa-fw", "fa-magnet");
    magnetLink.appendChild(magnetIcon);
    // Size
    let size = document.createElement("td");
    size.classList.add("text-center");
    size.textContent = obj.size;
    newElement.appendChild(size);
    // Date
    let timestamp = document.createElement("td");
    timestamp.classList.add("text-center")
    timestamp.setAttribute("data-timestamp", obj.date);
    timestamp.title = timeSince(obj.date);
    timestamp.textContent = formatTimestamp(obj.date);
    newElement.appendChild(timestamp);
    // Seeders
    let seeders = document.createElement("td");
    seeders.classList.add("text-center");
    seeders.textContent = obj.seeders;
    newElement.appendChild(seeders);
    // Leechers
    let leechers = document.createElement("td");
    leechers.classList.add("text-center");
    leechers.textContent = obj.leechers;
    newElement.appendChild(leechers);
    // Seeders
    let completed = document.createElement("td");
    completed.classList.add("text-center");
    completed.textContent = obj.completed;
    newElement.appendChild(completed);
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

function getSortOptions() {
    let row = document.querySelector("#sorting");
    let sortingDesc = false;
    let sortingButtons = document.querySelectorAll("#sorting>th:has(a)");
    for (let i = 0; i < sortingButtons.length; i++) {
        if (sortingButtons[i].classList.contains("sorting_desc")) {
            sortingDesc = true;
            break;
        }
    }
    let options = `&order=${(sortingDesc) ? "Descending" : "Ascending"}`;
    if (row.getAttribute("sort") == "hdr-date") {
        options += "&sort=Date";
    } else if (row.getAttribute("sort") == "hdr-size") {
        options += "&sort=Size";
    } else if (row.getAttribute("sort") == "hdr-comments") {
        options += "&sort=Comments";
    } else if (row.getAttribute("sort") == "hdr-seeders") {
        options += "&sort=Seeders";
    } else if (row.getAttribute("sort") == "hdr-leechers") {
        options += "&sort=Leechers";
    } else if (row.getAttribute("sort") == "hdr-downloads") {
        options += "&sort=Downloads";
    }
    return options;
}

function sortingButtonClicked(event) {
    let row = event.target.closest("tr");
    let button = event.target.closest("th");
    row.setAttribute("sort", button.classList[0]);
    let newSort = !button.classList.contains("sorting_desc");
    button.classList.remove((!newSort) ? "sorting_desc" : "sorting");
    button.classList.add((newSort) ? "sorting_desc" : "sorting");
    let sortingButtons = document.querySelectorAll("#sorting>th:has(a)");
    for (let i = 0; i < sortingButtons.length; i++) {
        if (sortingButtons[i] != button) {
            sortingButtons[i].classList.remove("sorting_desc");
            sortingButtons[i].classList.add("sorting");
        }
    }
    let options = getSortOptions();
    update_page(get_current_page(), options);
}

function setQuery(query) {
    let searchBar = document.querySelector(".search-bar");
    searchBar.value = query;
}

function setFilter(filter) {
    let idx = 1;
    if (filter == "None" || filter == "") {
        filter = "No filter";
    } else if (filter == "NoRemakes") {
        filter = "No remakes";
        idx = 2;
    } else {
        filter = "Trusted only";
        idx = 3;
    }
    document.querySelector("#navFilter-criteria>div>button").setAttribute("title", filter);
    document.querySelector("#navFilter-criteria>div>button>span.filter-option").textContent = filter;
    document.querySelector("#navFilter-criteria>div>div.dropdown-menu>ul>li.selected").classList.remove("selected");
    document.querySelector(`#navFilter-criteria>div>div.dropdown-menu>ul>li[data-original-index="${idx}"]`).classList.add("selected");
    document.querySelector("#navFilter-criteria>div>select>option[selected]").removeAttribute("selected");
    document.querySelector(`#navFilter-criteria>div>select>option[value="${idx - 1}"]`).setAttribute("selected", "selected");
}

function setCategory(category) {
    if (category == 0) {
        category = "0_0";
    } else {
        category = `${category.toString()[0]}_${category.toString()[1]}`;
    }
    let title = document.querySelector(`#navFilter-category>div>select>option[value="${category}"]`).getAttribute("title");
    let options = document.querySelectorAll("#navFilter-category>div>select>option[value*=\"_\"]");
    let idx = 1;
    for (let i = 0; i < options.length; i++) {
        if (options[i].getAttribute("value") == category) {
            idx = i + 1;
            break;
        }
    }
    document.querySelector("#navFilter-category>div>button").setAttribute("title", title);
    document.querySelector("#navFilter-category>div>button>span.filter-option").textContent = title;
    document.querySelector("#navFilter-category>div>div.dropdown-menu>ul>li.selected").classList.remove("selected");
    document.querySelector(`#navFilter-category>div>div.dropdown-menu>ul>li[data-original-index="${idx}"]`).classList.add("selected");
    document.querySelector("#navFilter-category>div>select>option[selected]").removeAttribute("selected");
    document.querySelector(`#navFilter-category>div>select>option[value="${category}"]`).setAttribute("selected", "selected");
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

window.addEventListener("load", async function () {
    let page_options = Object.fromEntries(new URLSearchParams(this.location.search));
    let init_page = (page_options.page != null) ? Number(page_options.page) : 1;
    let init_query = (page_options.query != null) ? page_options.query : "";
    let init_filter = (page_options.filter != null) ? page_options.filter : "None";
    let init_category = (page_options.category != null) ? Number(page_options.category) : 0;
    setQuery(init_query);
    setFilter(init_filter);
    setCategory(init_category);
    let options = (init_query != "") ? `&query=${init_query}` : "";
    options = (init_filter != "None") ? `${options}&filter=${init_filter}` : options;
    options = (init_category != 0) ? `${options}&category=${init_category}` : options;
    await update_page_buttons(init_page, options.replace("&", "?"));
    fetch_torrents(init_page, options);
    let sortingButtons = document.querySelectorAll("#sorting>th:has(a)");
    for (let i = 0; i < sortingButtons.length; i++) {
        sortingButtons[i].onclick = sortingButtonClicked;
    }
    let searchButton = document.querySelector(".search-btn>button");
    searchButton.onclick = function () {
        let queryOptions = getQueryOptions();
        let options = `${getSortOptions()}${(queryOptions !== "") ? `&${queryOptions}` : ''}`;
        update_page(1, options);
        update_page_buttons(1, `${(queryOptions != '') ? `?${queryOptions}` : ''}`);
        let parent = document.querySelector("body>div.container>div.center");
        if (queryOptions != "") {
            let info = parent.querySelector(".pagination-page-info");
            if (info === null) {
                info = document.createElement("div");
                info.classList.add("pagination-page-info");
            }
            info.textContent = "Displaying results 0-0 out of 0 results.\nPlease refine your search results if you can't find what you were looking for.";
            parent.appendChild(info);
        } else if (queryOptions == "" && parent.querySelector(".pagination-page-info") !== null) {
            parent.querySelector(".pagination-page-info").remove();
        }
        return false;
    }
});
