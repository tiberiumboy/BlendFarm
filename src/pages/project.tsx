import { invoke } from "@tauri-apps/api/tauri";
import ProjectFile from "./project_file";
import { useState } from "react";

export default function Project() {
  const [collection, setCollection] = useState([ProjectFile]);
  // here we will hold the application context and inforamtion to make modification
  // this is where we will store our data state
  // and information across the tools we expose.

  async function addtoProjectList() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    await invoke("add_project");
    // let _col = collection;
    // _col.push(_newFile);
    // setCollection(_col);
  }

  // TODO: replace any to strongly typed value
  async function editProject(id: any) {
    // todo - find a way to pass argument here and what kind of details do we need? Can we parse an object?
    await invoke("edit_project", id);
  }

  // Todo find a way to load previous project settings here!
  async function loadProjectList() {
    let message = await invoke("load_project_list");
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
        {/* <ProjectFile
          title="nuke_scene"
          src="/Users/jbejar/Shared/nuke_scene.blend"
          tmp="/var/folders/n6/71jhxnt54vlfx73_jypx3dvh0000gn/T/nuke_scene.blend"
        /> */}
        {collection.map((file) => (
          <ProjectFile
            id={file.id}
            title={file.title}
          // edit={editProject}
          />
        ))}
      </div>
    </div>
  );
}
