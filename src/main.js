const { invoke } = window.__TAURI__.tauri;

let input;
let logs;

async function loadBlendFile() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  let fReader = new FileReader();
  fReader.readAsDataURL(input.files[0]);
  fReader.onloadend = function (event) {
    logs.textContent = event.target.result;
  }

  // logs.textContent = 
  await invoke("load_blend_file", { name: input.value });
}

window.addEventListener("DOMContentLoaded", () => {
  input = document.getElementById("file-input");
  logs = document.querySelector("#msg-log");
  document.querySelector("#file-form").addEventListener("submit", (e) => {
    e.preventDefault();
    loadBlendFile();
  });
});
