async function sendLoginInfo(formData) {
    try {
        const request = await fetch("/api/login", {
            method: "POST",
            body: new URLSearchParams(formData)
        });
        if (!request.ok) {
            throw new Error(`Response status: ${request.status}`);
        }
        const response = await request.json();
        handleLoginResponse(response);
    } catch (error) {
        console.error(error);
    }
}

function handleLoginResponse(response) {
    let expiry = new Date(response.expiration * 1000);
    document.cookie = `token=${response.token};expires=${expiry.toUTCString()};path=/`;
    window.location.href = "/";
}

function getLoginInfo() {
    let username = document.querySelector("#username").value;
    let password = document.querySelector("#password").value;
    const formData = {
        username: username,
        password: password,
    };
    sendLoginInfo(formData);
}

window.addEventListener("load", function() {
    document.querySelector("#login").addEventListener("submit", (e) => {
        e.preventDefault();
        getLoginInfo();
        return false;
    })
});
