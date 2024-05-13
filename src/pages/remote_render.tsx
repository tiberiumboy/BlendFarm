import { open } from "@tauri-apps/api/dialog";
import { invoke } from "@tauri-apps/api/tauri";
import { listen, once } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import RenderJob, { RenderJobProps } from "../components/render_job";
import ProjectFile, { ProjectFileProps } from "../components/project_file";
import RenderNode, { RenderNodeProps } from "../components/render_node";

// TODO: Find a way to invoke global event updates so that we can notify image changes/updates
// hmm?
interface RenderComposedPayload {
  id: string;
  src: string;
}

// hmmmm?
const unlisten = await once<RenderComposedPayload>("image_update", (event) => {
  console.log(event);
});

// must deserialize into this format: "Frame": "i32",
const Frame = () => {
  <div>
    <label>Frame</label>
    <input name="frame" id="frame" type="number" />
  </div>;
};

// TODO: Find a good reason why we would prefer animation over section?
// must deserialize into this format: "Animation",
const Animation = () => {
  <div>Animation</div>;
};

// must deserialize into this format: "Section": { "start": i32, "end": i32 }
const Section = () => {
  <div>
    Section
    <label>Start</label>
    <input name="start" id="start" type="number" />
    <label>End</label>
    <input name="end" id="end" type="number" />
  </div>;
};

const components = {
  frame: Frame,
  animation: Animation,
  section: Section,
};

export default function RemoteRender() {
  const [projects, setProjects] = useState(fetchProjects);
  const [jobs, setJobs] = useState(fetchJobs);
  const [nodes, setNodes] = useState(fetchNodes);
  const [mode, setMode] = useState(components["frame"]);

  //#region User selected data
  const [selectedProject, setSelectedProject] = useState(
    {} as ProjectFileProps,
  );
  const [selectedNodes, setSelectedNodes] = useState([] as RenderNodeProps[]);
  //#endregion

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

  function fetchNodes() {
    listNodes();
    return [] as RenderNodeProps[];
  }

  //#endregion

  //#region API Calls to fetch Data
  function listProjects() {
    invoke("list_projects").then((ctx) => setProjects(JSON.parse(ctx + "")));
  }

  function listJobs() {
    invoke("list_job").then((ctx) => setJobs(JSON.parse(ctx + "")));
  }

  function listNodes() {
    invoke("list_node").then((ctx) => setNodes(JSON.parse(ctx + "")));
  }

  //#endregion

  //#region Dialogs
  function showDialog() {
    let dialog = document.getElementById("create_process");
    // TODO: Find a better way to fix this?
    dialog.showModal();
  }

  function closeDialog() {
    let dialog = document.getElementById("create_process");
    dialog.close();
  }

  const onCheckboxChanged = (node: RenderNodeProps) => {
    let data = selectedNodes;
    if (data.indexOf(node) == -1) {
      data.push(node);
    } else {
      data = data.filter((item) => item !== node);
    }
    setSelectedNodes(data);
    console.log("I've been changed!", selectedNodes);
  };

  function handleSubmitJobForm(e: any) {
    e.preventDefault();
    // TODO: Find a way to parse/serialize mode version
    let data = {
      output: e.target.output.value,
      projectFile: selectedProject,
      nodes: selectedNodes,
      mode: { Frame: 1 },
    };
    console.log(selectedNodes);
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

  function jobCreationDialog() {
    /*
      Display this window with a list of available nodes to select from,
      TODO: List blender version for the blender project we collected
      TODO: Test argument passing to rust and verify all system working as intended.

      once that is completed, it set forth a new queue instruction to all selected nodes.
      Send the project file for each selected nodes.
      Then, invoke blender with configurations (which frames) to the downloaded project file.
      Once blender completed, transfer result image back to the server.
      The host will display received image progress.
    */
    return (
      <dialog id="create_process">
        <form method="dialog" onSubmit={handleSubmitJobForm}>
          <h1>Dialog</h1>
          <label>Choose Node</label>
          {/* Toggle node checkboxes */}
          <label />
          Toggle nodes:
          <input
            id="toggleNodes"
            type="checkbox"
            onChange={(e: any) => {
              // Still skeptical about what Copilot writes here, but verify it afterward
              // const checkboxes = document.querySelectorAll(
              //   "input[type=checkbox]",
              // ); // TODO: dangerous wildcard here...
              // TODO: How do I send notification about update all set?
              if (e.target.checked) {
                setSelectedNodes(nodes);
              } else {
                setSelectedNodes([]);
              }
              // checkboxes.forEach((checkbox) => {

              //   checkbox.checked = e.target.checked;
              // });
            }}
          />
          {/* Checklist list I need to find a way to fetch nodes from other component! How?*/}
          {nodes.map((node: RenderNodeProps, index: number) => (
            <div key={"Node_" + node.name + "_" + index}>
              <label>{node.name}</label>
              <input
                type={"checkbox"}
                checked={selectedNodes.indexOf(node) != -1}
                onChange={() => onCheckboxChanged(node)}
              />
            </div>
          ))}
          <label>Choose rendering mode</label>
          <select
            name="modes"
            id="modes"
            onChange={(e) => {
              const component = components[e.target.value];
              setMode(component);
            }}
          >
            {Object.entries(components).map((item) => (
              <option value={item[0]}>{item[0]}</option>
            ))}
          </select>
          {mode}
          {/* Output field */}
          <input
            type="text"
            placeholder="Output Path"
            id="output"
            name="output"
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
            <button
              type="button"
              value="cancel"
              onClick={() => closeDialog("create_process")}
            >
              Cancel
            </button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>
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
