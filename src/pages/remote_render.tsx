import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen, once } from "@tauri-apps/api/event";
import { ChangeEvent, useEffect, useState } from "react";
import RenderJob, { RenderJobProps } from "../components/render_job";
import ProjectFile, { ProjectFileProps } from "../components/project_file";
import RenderNode, { RenderNodeProps } from "../components/render_node";

// TODO: Find a way to invoke global event updates so that we can notify image changes/updates
// hmm?
interface RenderComposedPayload {
  id: string;
  src: string;
}

// TODO: Figure out if this works or not, Need to re-read Tauri documentation again to understand event bridge between frontend and backend
const unlisten = await once<RenderComposedPayload>("image_update", (event) => {
  console.log(event);
});

// must deserialize into this format: "Frame": "i32",
const Frame = () => (
  <div>
    <label htmlFor="frame">Frame</label>
    <input name="frame" type="number" />
  </div>
);

// must deserialize into this format: "Section": { "start": i32, "end": i32 }
const Section = () => (
  <div key="frameRangeEntry">
    Section
    <label key="frameStartLabel" htmlFor="start">Start</label>
    <input key="frameStartField" name="start" type="number" value={1} />
    <label key="frameEndLabel" htmlFor="end">End</label>
    <input key="frameEndField" name="end" type="number" value={2} />
  </div>
);

const components = {
  frame: Frame,
  section: Section,
};

export default function RemoteRender() {
  // something went terribly wrong here?
  const [projects, setProjects] = useState(fetchProjects);
  const [jobs, setJobs] = useState(fetchJobs);
  const [versions, setVersions] = useState(fetchVersions);
  const [mode, setMode] = useState(components["frame"]());

  const [selectedProject, setSelectedProject] = useState(
    {} as ProjectFileProps,
  );

  useEffect(() => {
    unlisten(); // hmm should this work?
  }, []);

  //#region Initialization
  function fetchProjects() {
    listProjects();
    return [] as ProjectFileProps[];
  }

  function fetchJobs() {
    listJobs();
    return [] as RenderJobProps[];
  }

  function fetchVersions() {
    listVersions();
    return [] as string[];
  }

  //#endregion

  //#region API Calls to fetch Data
  function listProjects() {
    invoke("list_projects").then((ctx) => {
      setProjects(JSON.parse(ctx + ""));
    });
  }

  function listJobs() {
    invoke("list_job").then((ctx) => setJobs(JSON.parse(ctx + "")));
  }

  function listVersions() {
    invoke("list_versions").then((ctx) => setVersions(JSON.parse(ctx + "")));
  }

  //#endregion

  //#region Dialogs
  function showDialog() {
    // Is there a way I could just reference this directly? Or just create a new component for this?
    let dialog = document.getElementById("create_process") as HTMLDialogElement;
    dialog?.showModal();
  }

  function closeDialog() {
    let dialog = document.getElementById("create_process") as HTMLDialogElement;
    dialog?.close();
  }

  function generateMode(mode: any, target: any) {
    switch (mode) {
      case "frame":
        return {
          Frame: Number(target.frame.value),
        };
      case "section":
        return {
          Section: {
            start: Number(target.start.value),
            end: Number(target.end.value),
          },
        };
      default:
        return {};
    }
  }

  const handleSubmitJobForm = (e: React.FormEvent) => {
    e.preventDefault(); // wonder if this does anything?
    // How do I structure this?
    const info = e.target as HTMLFormElement;
    const selectedMode = info.modes.value;
    let mode = generateMode(selectedMode, e.target);
    let data = {
      output: info.output.value,
      version: info.version.value,
      projectFile: selectedProject,
      mode,
    };

    invoke("create_job", data).then(listJobs);
    closeDialog();
  }

  function openJobWindow(project: ProjectFileProps) {
    setSelectedProject(project);
    showDialog();
  }
  //#endregion

  //#region Display Components

  function projectWindow() {
    /*
     The goal behind this is to let the user import the projects into their own temp collection,
     which will be used to distribute across other nodes on the network.
     In this transaction - we take a copy of the source file, and put it into blenderFiles directory.
     Feature: This will be used as a cache to validate if the file has changed or not.

     Next, if the user clicks on the collection entry, we display a pop up asking which render node would this project like to use.
     Then any specific rendering configurations, E.g. Single frame or Animation
     We could utilize segment renderings where for a single frame, we take chunks of render and assemble them together for
     high resolution distribution job.
      */
    return (
      <div>
        <h2>Project List</h2>
        <button
          onClick={(e: any) => {
            e.preventDefault();
            open({
              multiple: true,
              filters: [
                {
                  name: "Blender",
                  extensions: ["blend"],
                },
              ],
            }).then((selected) => {
              if (Array.isArray(selected)) {
                // user selected multiple of files
                selected.forEach((entry) => {
                  invoke("import_project", { path: entry });
                });
              } else if (selected != null) {
                // user selected single file
                invoke("import_project", { path: selected });
              }
              listProjects();
            });
          }}
        >
          Import
        </button>
        <div className="group">
          {projects.map(
            (project: ProjectFileProps) => (
              (project.onDataChanged = listProjects),
              (project.onRequestNewJob = openJobWindow),
              ProjectFile(project)
            ),
          )}
        </div>
      </div>
    );
  }

  function jobWindow() {
    return (
      <div>
        {/* Collection of active job list */}
        <h2>Job List</h2>
        <div className="group">
          {jobs.map(
            (job: RenderJobProps) => (
              (job.onDataChanged = listJobs), RenderJob(job)
            ),
          )}
        </div>
      </div>
    );
  }

  function handleRenderModeChange(e: ChangeEvent<HTMLSelectElement>) {
    const index = parseInt(e.target.value);
    const mode = components[index]() as JSX.Element;
    setMode(mode);
  }

  // This should really be a separate React component for handling custom HTMLDialogElement
  function jobCreationDialog() {
    /*
      Display this window with a list of available nodes to select from,
      TODO: List blender version for the blender project we collected
      TODO: Test argument passing to rust and verify all system working as intended.

      once that is completed, it set forth a new queue instruction to all nodes.
      Send the project file for each nodes available on the network.
      Then, invoke blender with configurations (which frames) to the downloaded project file.
      Once blender completed, transfer result image back to the server.
      The host will display received image progress.
    */
    return (
      <dialog id="create_process">
        <form method="dialog" onSubmit={handleSubmitJobForm}>
          <h1>Create new Render Job</h1>
          <label>Choose rendering mode</label>
          <select name="modes" onChange={handleRenderModeChange} >
            {Object.entries(components).map((item) => (
              <option value={item[0]}>{item[0]}</option>
            ))}
          </select>
          <br />
          <label>Blender Version:</label>
          {/* TODO: Find a way to fetch default blender version by user preference? */}
          <select name="version" value={"4.1.0"}>
            {versions.map((item) => (
              <option value={item}>{item}</option>
            ))}
          </select>
          {mode}
          <label>Output destination:</label>
          <input
            type="text"
            placeholder="Output Path"
            id="output"
            name="output"
            value={"/Users/Shared/"}
            readOnly={true}
            onClick={async (e: any) => {
              const filePath = await open({
                directory: true,
                multiple: false,
              });
              if (filePath != null) {
                // TODO: find a way to include the dash elsewhere
                e.target.value = filePath + "/";
              }
            }}
          />
          <menu>
            <button type="button" value="cancel" onClick={closeDialog}>
              Cancel
            </button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog >
    );
  }

  //#endregion

  return (
    <div className="content">
      <h1>Remote Render</h1>
      {projectWindow()}
      {jobWindow()}
      {jobCreationDialog()}
    </div>
  );
}
