async function sendRegisterInfo(formData) {
    try {
        const request = await fetch("/api/register", {
            method: "POST",
            body: new URLSearchParams(formData)
        });
        if (!request.ok) {
            throw new Error(`Response status: ${request.status}`);
        }
        const response = await request.json();
        handleRegisterResponse(response);
    } catch (error) {
        console.error(error);
    }
}

function handleRegisterResponse(response) {
    let expiry = new Date(response.expiration * 1000);
    document.cookie = `token=${response.token}; expires=${response.expiration}; path=/`;
    window.location.href = "/";
}

function getRegisterInfo() {
    let username = document.querySelector("#username").value;
    let password = document.querySelector("#password").value;
    let password_confirm = document.querySelector("#password_confirm").value;
    const formData = {
        username: username,
        password: password,
        password_confirm: password_confirm
    };
    sendRegisterInfo(formData);
}

window.addEventListener("load", function() {
    document.querySelector("#register").addEventListener("submit", (e) => {
        e.preventDefault();
        getRegisterInfo();
        return false;
    })
});
