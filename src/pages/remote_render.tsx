import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { ChangeEvent, useState } from "react";
import RenderJob, { RenderJobProps } from "../components/render_job";

// TODO: Figure out if this works or not, Need to re-read Tauri documentation again to understand event bridge between frontend and backend
// const unlisten = await once<RenderComposedPayload>("image_update", (event) => {
//   console.log(event);
// });

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

const components: any = {
  frame: Frame,
  section: Section,
};

function JobDetail(prop: { job: RenderJobProps | undefined }) {
  if (prop.job != null) {
    return (
      < div >
        <h2>Job Details: {prop.job.id}</h2>
        <p>File name: {prop.job.project_file.file_name}</p>
        <p>Status: Finish</p>
        <p>Progress: 100/100%</p>
        {/* Find a way to pipe the image here? or call fetch the last image received */}
        <img src={prop.job.renders[0]} />
      </div >
    )
  } else {
    return (
      <div>
        <p>
          <i>
            Please select a job above to see the full content.
          </i>
        </p>
      </div>
    )
  }
}

function JobCreationDialog(versions: string[], jobCreated: (job: RenderJobProps) => void) {
  const [mode, setMode] = useState(components["frame"]());
  const [version, setVersion] = useState(versions[0]);

  const handleSubmitJobForm = (e: React.FormEvent) => {
    e.preventDefault();
    // How do I structure this?
    const info = e.target as HTMLFormElement;
    const selectedMode = info.modes.value;
    const filePath = info.file_path.value;
    const output = info.output.value;

    let mode = generateMode(selectedMode, e.target);

    let data = {
      filePath,
      output,
      version,
      // mode,
    };

    console.log(data);

    invoke("create_job", data).then((ctx: any) => {
      if (ctx == null) {
        return;
      }
      jobCreated(ctx as RenderJobProps); // chances are it could be invalid data? todo; unit test this?
    });
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

  // TODO: find a way to make this more sense and pure function as possible.
  async function onDirectorySelect(e: any) {
    const filePath = await open({
      directory: true,
      multiple: false,
    });
    if (filePath != null) {
      // TODO: find a way to include the dash elsewhere
      e.target.value = filePath + "/";
    }
  }

  async function onFileSelect(e: any) {
    const filePath = await open({
      directory: false,
      multiple: false,
      filters: [
        {
          name: "Blender",
          extensions: ["blend"],
        },
      ],
    })
    e.target.value = filePath;
  }

  /*
      Display this window with a list of available nodes to select from,
      TODO: Test argument passing to rust and verify all system working as intended.
  
      once that is completed, it set forth a new queue instruction to all nodes.
      Send the project file for each nodes available on the network.
      Then, invoke blender with configurations (which frames) to the downloaded project file.
      Once blender completed, transfer result image back to the server.
      The host will display received image progress. 
      Feature: It would be nice to stream render image input from any computer node. See their rendering progress.
    */
  return (
    /**
     * TODO: Change the process so that we instead ask the user to open the .blend file
     * then with the backend service to parse the .blend file we can extract information 
     * Once we get that info - we display the create_process dialog to display the information provided by the blend file.
     */
    <dialog id="create_process">
      <form method="dialog" onSubmit={handleSubmitJobForm}>
        <h1>Create new Render Job</h1>
        <label>Project File Path:</label>
        <input type="text" placeholder="Project path" id="file_path" name="file_path" readOnly={true} onClick={onFileSelect} />
        <br />
        <label>Choose rendering mode</label>
        <select name="modes" onChange={handleRenderModeChange} >
          {Object.entries(components).map((item) => (
            <option value={item[0]}>{item[0]}</option>
          ))}
        </select>
        <br />
        <label>Blender Version:</label>
        {/* TODO: Find a way to fetch default blender version by user preference? */}
        <select value={version} onChange={(e) => setVersion(e.target.value)}>
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
          onClick={onDirectorySelect}
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

export interface RemoteRenderProps {
  versions: string[];
  jobs: RenderJobProps[];
  onJobCreated(job: RenderJobProps): void;
}

export default function RemoteRender(props: RemoteRenderProps) {
  const [selectedJob, setSelectedJob] = useState<RenderJobProps>();

  //#region Dialogs
  function showDialog() {
    // Is there a way I could just reference this directly? Or just create a new component for this?
    // TOOD: Invoke rust backend service to open dialog and then parse the blend file
    // if the user cancel or unable to parse - return a message back to the front end explaining why
    // Otherwise, display the info needed to re-populate the information.
    invoke("import_blend").then((ctx) => {
      if (ctx == null) {
        return;
      }
      // I'm always curious about this code.
      let data = JSON.parse(ctx as string);
      console.log(ctx, data);
    })
    let dialog = document.getElementById("create_process") as HTMLDialogElement;
    dialog?.showModal();
  }

  function onJobSelected(job: RenderJobProps): void {
    setSelectedJob(job);
  }

  return (
    <div className="content">
      <h2>Remote Jobs</h2>
      <button onClick={showDialog}>
        Import
      </button>
      {/* Collection of active job list */}
      <div className="group">
        {props.jobs.map((job) => RenderJob(job, onJobSelected))}
      </div>

      <JobDetail job={selectedJob} />

      {JobCreationDialog(props.versions, props.onJobCreated)}
    </div>
  );
}
