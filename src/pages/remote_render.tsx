import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { ChangeEvent, useState } from "react";
import RenderJob, { GetFileName, RenderJobProps } from "../components/render_job";
import { listen } from "@tauri-apps/api/event";

// TODO: Have a look into channels: https://v2.tauri.app/develop/calling-frontend/#channels
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
        <p>File name: {GetFileName(prop.job.project_file)}</p>
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

export interface RemoteRenderProps {
  versions: string[];
  jobs: RenderJobProps[];
  onJobCreated(job: RenderJobProps): void;
}

const unlisten = await listen("version-update", (event) => {
  console.log(event);
})

export default function RemoteRender(props: RemoteRenderProps) {
  const [selectedJob, setSelectedJob] = useState<RenderJobProps>();
  const [path, setPath] = useState<string>("");
  const [version, setVersion] = useState<string>("");
  const [mode, setMode] = useState(components["frame"]());

  //#region Dialogs
  async function showDialog() {
    // Is there a way I could just reference this directly? Or just create a new component for this?
    // TOOD: Invoke rust backend service to open dialog and then parse the blend file
    // if the user cancel or unable to parse - return a message back to the front end explaining why
    // Otherwise, display the info needed to re-populate the information.
    const file_path = await open({
      directory: false,
      multiple: false,
      filters: [
        {
          name: "Blender",
          extensions: ["blend"],
        },
      ],
    });

    if (file_path == null) {
      return;
    }

    invoke("import_blend", { path: file_path }).then((ctx) => {
      if (ctx == null) {
        return;
      }

      // TODO: For future impl. : We will try and read the file from the backend to extract information to show the user information about the blender
      // then we will populate those data into the dialog form, allowing user what BlendFarm sees, making any last adjustment before creating a new job.
      let data = JSON.parse(ctx as string);
      setPath(file_path);
      setVersion(data.blend_version);
      openDialog();
    })
  }

  function onJobSelected(job: RenderJobProps): void {
    setSelectedJob(job);
  }

  const handleSubmitJobForm = (e: React.FormEvent) => {
    e.preventDefault();

    // How do I structure this?
    const info = e.target as HTMLFormElement;
    const selectedMode = info.modes.value;
    const path = info.file_path.value;
    const output = info.output.value;

    let mode = generateMode(selectedMode, e.target);
    let data = {
      mode,
      version,
      path,
      output,
    };

    invoke("create_job", data).then((ctx: any) => {
      if (ctx == null) {
        return;
      }
      console.log("After create_job post", ctx);

      let data: RenderJobProps = {
        current_frame: 0,
        id: ctx.job.id,
        mode: ctx.job.mode,
        output: ctx.output,
        project_file: ctx.job.project_file,
        renders: [],
        version: ctx.job.blender_version,
      };
      console.log(data);
      props.onJobCreated(data);
    });
    closeDialog();
  }

  function openDialog() {
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

  function handleRenderModeChange(e: ChangeEvent<HTMLSelectElement>) {
    const index = parseInt(e.target.value);
    const mode = components[index]() as JSX.Element;
    setMode(mode);
  }

  // TODO: find a way to make this more sense and pure function as possible.
  // see if I can just invoke a rust backend to handle file directory or file open instead?
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

  return (
    <div className="content">
      <h2>Remote Jobs</h2>
      {/* How can I enable hotkey function for html code? */}
      <button onClick={showDialog}>
        Import
      </button>
      {/* Collection of active job list */}
      <div className="group">
        {props.jobs.map((job) => RenderJob(job, onJobSelected))}
      </div>

      <JobDetail job={selectedJob} />

      <dialog id="create_process">
        <form method="dialog" onSubmit={handleSubmitJobForm}>
          <h1>Create new Render Job</h1>
          <label>Project File Path:</label>
          <input type="text" value={path} placeholder="Project path" id="file_path" name="file_path" readOnly={true} />
          <br />
          <label>Choose rendering mode</label>
          <select name="modes" onChange={handleRenderModeChange}>
            {Object.entries(components).map((item) => (
              <option value={item[0]}>{item[0]}</option>
            ))}
          </select>
          <br />
          <label>Blender Version:</label>
          <select value={version} onChange={(e) => setVersion(e.target.value)}>
            {props.versions.map((item) => (
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
            value={"/Users/Shared/"}  // change this?
            readOnly={true}
            onClick={onDirectorySelect}
          />
          <menu>
            <button type="button" value="cancel" onClick={closeDialog}>Cancel</button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>
    </div>
  );
}
