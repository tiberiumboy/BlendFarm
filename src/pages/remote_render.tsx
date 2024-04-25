import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { open } from "@tauri-apps/api/dialog";
import RenderNode, { RenderNodeProps } from "../components/render_node";
import RenderJob, { RenderJobProps } from "../components/render_job";

export default function RemoteRender() {
  const [nodes, setNodes] = useState([]);
  const [jobs, setJobs] = useState([]);
  useEffect(() => {
    listNode();
    listJob();
  }, []);

  function CreateNewJob(e: any) {
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
      console.log(selected);
      if (Array.isArray(selected)) {
        // user selected multiple of files
        selected.forEach((entry) => {
          invoke("create_job", { path: entry });
        });
      } else if (selected != null) {
        invoke("create_job", { path: selected });
        // user selected single file
      }
      listJob();
    });

    // Problem here, backend is invoked via async, and will execute file launcher.
    // Find a way to invoke file launcher on javascript side instead, and then rely on using the backend to handle the path obtained from front end.
    // then we can delegate invoking proper event to handle project file creation.
    return false;
  }

  // TODO: See if we can refactor this? I don't like the fact that we hardcode port value here.'
  function handleSubmit(e: any) {
    e.preventDefault();
    let data = {
      name: e.target.name.value,
      host: e.target.ip.value + ":15000",
    };
    invoke("create_node", data).then(listNode);
    closeCreateNew("create_node");
  }

  function listNode() {
    invoke("list_node").then((ctx) => {
      let data = JSON.parse(ctx + "");
      setNodes(data);
    });
  }

  function listJob() {
    invoke("list_job").then((ctx) => {
      let data = JSON.parse(ctx + "");
      setJobs(data);
    });
  }

  function deleteNode(id: any) {
    invoke("delete_node", { id }).then(() => {
      let col = nodes.filter((node: any) => node.id != id);
      setNodes(col);
    });
  }

  function showCreateNew(id: string) {
    let dialog = document.getElementById(id);
    dialog.showModal();
  }

  function closeCreateNew(id: string) {
    let dialog = document.getElementById(id);
    dialog.close();
  }

  return (
    <div className="content">
      <h1>Remote Render</h1>

      {/* Show the activity of the computer progress */}
      <h2>Computer Nodes</h2>
      <button onClick={() => showCreateNew("create_node")}>Create New</button>
      <br></br>
      <h4>
        <div className="group" id="RenderNodes">
          {nodes.map((node: RenderNodeProps) => (
            // A bit far fetch here, but can we rename nodes? Or edit it?
            <RenderNode
              id={node.id}
              key={node.id}
              name={node.name}
              onDataChanged={listNode} // wonder what this one was suppose to be?
            />
          ))}
        </div>
      </h4>

      {/* Collection of active job list */}
      <h2>Job List</h2>

      <button onClick={CreateNewJob}>Create new Job</button>
      <br></br>
      <h4>
        <div className="group">
          {jobs.map((job: RenderJobProps) => (
            <RenderJob id={job.id} project_file={job.project_file} />
          ))}
        </div>
      </h4>

      <dialog id="create_job">
        <form method="dialog" onSubmit={handleSubmit}>
          <h1>Dialog</h1>
          <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" />

          <menu>
            <button
              type="button"
              value="cancel"
              onClick={() => closeCreateNew("create_job")}
            >
              Cancel
            </button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>

      <dialog id="create_node">
        <form method="dialog" onSubmit={handleSubmit}>
          <h1>Dialog</h1>
          <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" />

          <menu>
            <button
              type="button"
              value="cancel"
              onClick={() => closeCreateNew("create_node")}
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
