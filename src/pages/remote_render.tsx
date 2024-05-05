import { open } from "@tauri-apps/api/dialog";
import { invoke } from "@tauri-apps/api/tauri";
import { listen, once } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import RenderJob, { RenderJobProps } from "../components/render_job";
import ProjectFile, { ProjectFileProps } from "../components/project_file";
import RenderNode, { RenderNodeProps } from "../components/render_node";
import Checkbox from "../components/Checkbox";

interface RenderComposedPayload {
  id: string;
  src: string;
}

const unlisten = await once<RenderComposedPayload>("image_update", (event) => {
  console.log(event);
});

export default function RemoteRender() {
  //#region Main data collection
  const [nodes, setNodes] = useState(fetchNodes);
  const [projects, setProjects] = useState(fetchProjects);
  const [jobs, setJobs] = useState(fetchJobs);
  const [preview, setPreview] = useState("");
  //#endregion

  //#region User selected data
  const [selectedNodes, setSelectedNodes] = useState([] as RenderNodeProps[]);
  const [selectedProject, setSelectedProject] = useState(
    {} as ProjectFileProps,
  );
  //#endregion

  useEffect(() => {
    unlisten(); // hmm should this work?
  }, []);

  //#region Initialization

  // TODO: Move nodes inside sidebar. Makes more sense to allow adding/removing nodes from there.
  function fetchNodes() {
    const initialNodes: RenderNodeProps[] = [];
    listNodes();
    return initialNodes;
  }

  function fetchProjects() {
    const initialProjects: ProjectFileProps[] = [];
    listProjects();
    return initialProjects;
  }

  function fetchJobs() {
    const initialJobs: RenderJobProps[] = [];
    listJobs();
    return initialJobs;
  }

  //#endregion

  //#region API Calls to fetch Data
  function listNodes() {
    invoke("list_node").then((ctx) => setNodes(JSON.parse(ctx + "")));
  }

  function listProjects() {
    invoke("list_projects").then((ctx) => setProjects(JSON.parse(ctx + "")));
  }

  function listJobs() {
    invoke("list_job").then((ctx) => setJobs(JSON.parse(ctx + "")));
  }

  //#endregion

  //#region Dialogs
  function showDialog(id: string) {
    let dialog = document.getElementById(id);
    // TODO: Find a better way to fix this?
    dialog.showModal();
  }

  function closeDialog(id: string) {
    let dialog = document.getElementById(id);
    dialog.close();
  }

  function handleSubmitNodeForm(e: any) {
    e.preventDefault();
    let data = {
      name: e.target.name.value,
      // TODO: remove magic hardcoded port value
      host: e.target.ip.value + ":15000",
    };
    invoke("create_node", data).then(listNodes);
    closeDialog("create_node");
  }

  function handleSubmitJobForm(e: any) {
    e.preventDefault();
    let data = {
      output: e.target.output.value,
      projectId: selectedProject.id,
      nodes: selectedNodes,
    };
    invoke("create_job", data).then(listJobs);
    closeDialog("create_process");
  }

  const onCheckboxChanged = (e: any, props: RenderNodeProps) => {
    let data = selectedNodes;

    if (e.target.checked) {
      data.push(props);
    } else {
      data = data.filter((node) => node.id !== props.id);
    }
    setSelectedNodes(data);
  };

  function openJobWindow(project: ProjectFileProps) {
    setSelectedProject(project);
    showDialog("create_process");
  }
  //#endregion

  //#region Display Components
  function nodeWindow() {
    return (
      <div>
        <h1>Remote Render</h1>

        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <button onClick={() => showDialog("create_node")}>Connect</button>
        <div className="group" id="RenderNodes">
          {nodes.map(
            (node: RenderNodeProps, index: Number) => (
              (node.onDataChanged = listNodes), RenderNode(index, node)
            ),
          )}
        </div>
      </div>
    );
  }

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
              ((job.onDataChanged = listJobs), (job.picture = preview)),
              RenderJob(job)
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
              const checkboxes = document.querySelectorAll(
                "input[type=checkbox]",
              ); // TODO: dangerous wildcard here...
              checkboxes.forEach((checkbox) => {
                checkbox.checked = e.target.checked;
              });
            }}
          />
          {/* Checklist list */}
          {nodes.map(
            (node: RenderNodeProps) => (
              (node.onDataChanged = onCheckboxChanged), Checkbox(node)
            ),
          )}
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

  function nodeCreationDialog() {
    return (
      <dialog id="create_node">
        <form method="dialog" onSubmit={handleSubmitNodeForm}>
          <h1>Dialog</h1>
          {/* <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" /> */}

          <menu>
            <button
              type="button"
              value="cancel"
              onClick={() => closeDialog("create_node")}
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
      {nodeWindow()}
      {projectWindow()}
      {jobWindow()}
      {jobCreationDialog()}
      {nodeCreationDialog()}
    </div>
  );
}
