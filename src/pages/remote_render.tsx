import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen, once } from "@tauri-apps/api/event";
import { ChangeEvent, useState } from "react";
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

async function sendProjectToBackend(entry: any, projects: ProjectFileProps[], setProjects: any): Promise<(ProjectFileProps | undefined)> {
  const ctx: any = await invoke("import_project", { path: entry });
  if (ctx === null) {
    return undefined;
  }
  return JSON.parse(ctx) as ProjectFileProps;
}

function ProjectWindow(onRequestJobWindow: (e: ProjectFileProps) => void) {
  const [projects, setProjects] = useState([] as ProjectFileProps[]);

  function listProjects() {
    console.log("list projects was called");
    invoke("list_projects").then((ctx: any) => {
      const data: ProjectFileProps[] = JSON.parse(ctx);
      setProjects(data);
    });
  }

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
          }).then(async (selected) => {
            let col = projects;
            if (Array.isArray(selected)) {
              selected.forEach(async (entry) => {
                let data = await sendProjectToBackend(entry, projects, setProjects);
                if (data !== undefined) { col.push(data); }
              });
            } else if (selected != null) {
              let data = await sendProjectToBackend(selected, projects, setProjects);
              if (data !== undefined) { col.push(data); }
            }

            setProjects(col);
            console.log("projects:", projects);
          });
        }}
      >
        Import
      </button>
      <div className="group">
        {projects.map(
          (project: ProjectFileProps) => (
            (project.onDataChanged = listProjects),
            (project.onRequestNewJob = onRequestJobWindow),
            ProjectFile(project)
          ),
        )}
      </div>
    </div >
  );
}

function JobWindow(jobs: RenderJobProps[]) {
  return (
    <div>
      {/* Collection of active job list */}
      <h2>Job List</h2>
      <div className="group">
        {jobs.map(RenderJob)}
      </div>
    </div>
  );
}

function JobCreationDialog(selectedProject: ProjectFileProps) {
  const [mode, setMode] = useState(components["frame"]());
  const [versions, setVersions] = useState([] as string[]);

  // how can I go about getting the list of blender version here?
  function listVersions() {
    invoke("list_versions").then((ctx: any) => {
      const data: string[] = JSON.parse(ctx);
      setVersions(data);
    });
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

    invoke("create_job", data); //.then(listJobs);
    closeDialog();
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

  function handleRenderModeChange(e: ChangeEvent<HTMLSelectElement>) {
    const index = parseInt(e.target.value);
    const mode = components[index]() as JSX.Element;
    setMode(mode);
  }


  /*
      Display this window with a list of available nodes to select from,
      TODO: List blender version for the blender project we collected
      TODO: Test argument passing to rust and verify all system working as intended.
  
      once that is completed, it set forth a new queue instruction to all nodes.
      Send the project file for each nodes available on the network.
      Then, invoke blender with configurations (which frames) to the downloaded project file.
      Once blender completed, transfer result image back to the server.
      The host will display received image progress. 
      Feature: It would be nice to stream render image input from any computer node. See their rendering progress.
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

export default function RemoteRender() {
  // something went terribly wrong here?
  const [jobs, setJobs] = useState([] as RenderJobProps[]);
  const [selectedProject, setSelectedProject] = useState(
    {} as ProjectFileProps,
  );

  //#region API Calls to fetch Data

  // function listJobs() {
  //   invoke("list_job").then((ctx: any) => {
  //     const data: RenderJobProps[] = JSON.parse(ctx);
  //     setJobs(data)
  //   });
  // }

  //#endregion

  //#region Dialogs
  function showDialog() {
    // Is there a way I could just reference this directly? Or just create a new component for this?
    let dialog = document.getElementById("create_process") as HTMLDialogElement;
    dialog?.showModal();
  }

  function openJobWindow(project: ProjectFileProps) {
    setSelectedProject(project);
    showDialog();
  }

  return (
    <div className="content">
      <h1>Remote Render</h1>
      {ProjectWindow(openJobWindow)}
      {JobWindow(jobs)}
      {JobCreationDialog(selectedProject)}
    </div>
  );
}
