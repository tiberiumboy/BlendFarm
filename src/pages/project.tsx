import { invoke } from "@tauri-apps/api/tauri";
import { ProjectFileProps } from "./project_file";
import ProjectFile from "./project_file";
import { useState, ReactNode } from "react";

export default function Project() {
  const [collection, setCollection] = useState([]);
  // here we will hold the application context and inforamtion to make modification
  // this is where we will store our data state
  // and information across the tools we expose.

  window.addEventListener("project_list", (msg) => {
    console.log("project_list", msg);
  });

  async function addtoProjectList() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    await invoke("add_pqroject");
    loadProjectList();
  }

  // TODO: replace any to strongly typed value
  // async function editProject(id: any) {
  //   // todo - find a way to pass argument here and what kind of details do we need? Can we parse an object?
  //   await invoke("edit_project", id);
  // }

  // Todo find a way to load previous project settings here!
  async function loadProjectList() {
    let message: string = await invoke("load_project_list");
    setCollection(JSON.parse(message));
    console.log("load_project_list", collection);
    // from here we can setCollection()
  }

  function drawProjectFileItem(file: ProjectFileProps, key: Number) {
    return ProjectFile(file, key);
  }

  function loadList() {
    return collection.map(drawProjectFileItem);
  }

  loadProjectList();

  return (
    <div className="content">
      <h3>Load Blender</h3>
      <button id="load_project" type="submit" onClick={addtoProjectList}>
        Load Blend file
      </button>

      {/* Show the list of project available here */}

      <div className="group" id="project-list">
        {loadList()}
      </div>
    </div>
  );
}
