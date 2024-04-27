import { open } from "@tauri-apps/api/dialog";
import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";
import RenderJob, { RenderJobProps } from "../components/render_job";
import ProjectFile, { ProjectFileProps } from "../components/project_file";
import RenderNode, { RenderNodeProps } from "../components/render_node";
import Checkbox from "../components/Checkbox";

export default function RemoteRender() {
  const [nodes, setNodes] = useState(fetchNodes);
  const [projects, setProjects] = useState(fetchProjects);
  const [jobs, setJobs] = useState(fetchJobs);

  const [selectedProject, setSelectedProject] = useState({});

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

  const importProject = (e: any) => {
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
  };

  // TODO: Find a better way than explicitly casting into string? How can I assign object type inside closure argument?
  function listNodes() {
    invoke("list_node").then((ctx) => setNodes(JSON.parse(ctx + "")));
  }

  function listProjects() {
    invoke("list_projects").then((ctx) => setProjects(JSON.parse(ctx + "")));
  }

  function listJobs() {
    invoke("list_job").then((ctx) => setJobs(JSON.parse(ctx + "")));
  }

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
      nodes: e.target.selectedNodes,
    };
    console.log(data);
    // invoke("create_job", data).then(listJobs);
    closeDialog("create_process");
  }

  async function showSaveDialog() {
    const filePath = await open({
      directory: true,
      multiple: false,
    });
    if (filePath != null) {
      document.getElementById("output").value = filePath;
    }
  }

  function openJobWindow(project: ProjectFileProps) {
    setSelectedProject(project);
    showDialog("create_process");
  }

  // const handleMultipleCheckboxChange = (event) => {

  // };

  // Components - trying to separate them to make it easy to read through this.
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
        <button onClick={importProject}>Import</button>
        <div className="group">
          {projects.map(
            (project: ProjectFileProps) => (
              ((project.onDataChanged = listProjects),
              (project.onRequestNewJob = openJobWindow)),
              ProjectFile(project)
            ),
          )}
        </div>
      </div>
    );
  }

  function jobWindow() {}

  return (
    <div className="content">
      {nodeWindow()}
      {projectWindow()}
      {jobWindow()}

      {/* Collection of active job list */}
      <h2>Job List</h2>
      <div className="group">
        {jobs.map(
          (job: RenderJobProps) => (
            (job.onDataChanged = listJobs), RenderJob(job)
          ),
        )}
      </div>

      {/* I no longer need create-job, instead, I need dialog to start a new process
          Display this window with a list of available nodes to select from,
          then let the operator chooses which blender version.
          once that is completed, it set forth a new queue instruction to all selected nodes.
          Send the project file for each selected nodes.
          Then, invoke blender with configurations (which frames) to the downloaded project file.
          Once blender completed, transfer result image back to the server.
          The host will display received image progress.
          */}
      <dialog id="create_process">
        <form method="dialog" onSubmit={handleSubmitJobForm}>
          <h1>Dialog</h1>
          <label>Choose Node</label>
          <input
            type="checkbox"
            onChange={(event) => {
              // handleMultipleCheckboxChange(event);
            }}
          />
          {nodes.map((node: RenderNodeProps, index: Number) => {
            return (
              <div key={"node_" + index}>
                <label>{node.name}</label>
                <input
                  type="checkbox"
                  id={node.id}
                  name={node.name}
                  value={node.id}
                />
              </div>
            );
          })}

          {/* find a way to open save dialog here? */}
          <input
            type="text"
            placeholder="Output Path"
            id="output"
            name="output"
            readOnly={true}
            onClick={showSaveDialog}
          />

          {/* <input type="text" placeholder="Name" id="name" name="name" /> */}

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
    </div>
  );
}
