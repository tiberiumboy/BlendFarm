function loadPage(url) {
  let xhr = new XMLHttpRequest();
  let content = document.getElementById("content");
  xhr.onreadystatechange = function () {
    if (xhr.readyState == 4 && xhr.status == 200) {
      content.innerHTML = xhr.responseText;
    }
  }

  xhr.open("GET", url, true);
  xhr.send(null);
}

window.addEventListener("DOMContentLoaded", () => {
  $("ul.nav-links li > a").click(function (e) {
    e.preventDefault();
    var url = $(this).attr("href");
    console.log(url);
    loadPage(url);
    return false;
    // loadPage(url);
  });

  loadPage("./project.html");
});
