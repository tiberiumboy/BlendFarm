import { invoke } from "@tauri-apps/api/tauri";
import { ProjectFileProps } from "../components/project_file";
import ProjectFile from "../components/project_file";
import { useState, useEffect } from "react";

export default function Project() {
  // TODO: Find out how I can explicitly set the state to accept ProjectFileProps[] instead?
  const [collection, setCollection] = useState([]);
  // here we will hold the application context and inforamtion to make modification
  // this is where we will store our data state and information across the tools we expose.
  // Look into using useState to call LoadProjectList once instead of every update refreshes

  window.addEventListener("project_list", (msg) => {
    console.log("project_list", msg);
  });

  async function addtoProjectList() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    // because we're not expecting anything return, dialog will continue to run in the backgrund async. Exit early before completion
    invoke("add_project"); //.then(loadProjectList);
  }

  useEffect(() => {
    loadProjectList();
  }, []);


  // Todo find a way to load previous project settings here!
  async function loadProjectList() {
    let message: string = await invoke("load_project_list");
    let col: ProjectFileProps[] = JSON.parse(message) as ProjectFileProps[];
    console.log(message);
    // need to figure out how I can fix this typecast issue?
    setCollection(col);
  }

  return (
    <div className="content">
      <h3>Load Blender</h3>
      <button id="load_project" type="submit" onClick={addtoProjectList}>
        Load Blend file
      </button>

      <div className="group" id="project-list">
        {collection.map((file: ProjectFileProps) =>
          <ProjectFile
            file_name={file.file_name}
            onDataChanged={loadProjectList}
          />
        )}
      </div>
    </div>
  );
}
