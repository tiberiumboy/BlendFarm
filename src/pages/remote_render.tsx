import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc, invoke } from "@tauri-apps/api/core"; 
import { useEffect, useRef, useState } from "react";
import RenderJob, { GetFileName, RenderJobProps } from "../components/render_job";
import { listen, TauriEvent } from "@tauri-apps/api/event";
import JobDialog from "../components/job_dialog";

// TODO: Have a look into channels: https://v2.tauri.app/develop/calling-frontend/#channels
function showImage(path: string) {
  if (path !== null) {
    return <img className="center-fit" src={convertFileSrc(path)}  />
  } else {
    return <div></div>
  }
}

function JobDetail(prop: { job: RenderJobProps | null }) {
  if (prop != null && prop.job != null) {
    const job = prop.job;
    return (
      <div>
        <h2>Job Details: {job.id}</h2>
        <p>File name: {GetFileName(job.project_file)}</p>
        <p>Status: Finish</p>
        <div className="imgbox">
          { showImage( job.renders[0]) }
        </div>
      </div>
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

export interface BlendInfo {
  path: string;
  blend_version: string;
  start: number;
  end: number;
  output: string;
}

// when using "withGlobalTauri": true - I can use this line below:
// const Database = window.__TAURI__.sql;

// const db = await Database.load("sqlite:blendfarm.db");


// export const getJobs = async ( db: Database): Promise<[]> => {
//   try {
//     // const jobs: Job[] = [];
//     const results = await db.execute("SELECT * FROM Jobs")
//     console.log(results);
//     // results?.forEach((result : any) => {
//     //   for( let index = 0; index < result.rows.length; index++ ) {
//     //     console.log(result)
//     //     // jobs.push(result.rows.item(index));
//     //   }
//     // })
//     // return jobs;
//     return [];
//   } catch( err ) {
//     console.log(err);
//     throw Error("Failed to get Jobs from Database");
//   }
// }

const droppedPath: string[] = [];

export default function RemoteRender(props: RemoteRenderProps) {

  const [selectedJob, setSelectedJob] = useState<RenderJobProps | null>(null);
  const [blendInfo, setBlendInfo] = useState<BlendInfo | null>(null);
  const dialogRef = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    if (!blendInfo) return;
    dialogRef.current?.showModal();
  }, [blendInfo])

  //#region Dialogs

  listen(TauriEvent.DRAG_DROP, event => { 
    const path: string = event.payload.paths[0];
    // this solution below was to prevent spamming commands to backend from front end. Currently, it's calling this function 8-9 times from a single file drop.
    if ( !droppedPath.includes(path)) {
      droppedPath.push(path);
      showDialog(path);
      // allow user to reupload another file after 100ms
      setTimeout(() => droppedPath.length = 0, 100);
    }
  });

  async function onImportClick() {
     // TOOD: Invoke rust backend service to open dialog and then parse the blend file
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

    showDialog(file_path);
  }
  
  /*  
    Reading the code below may seem confusing so it's best to explain here -
    We want to fetch data out of blender file before displaying information to the user.
    This process automates prepopulating field many artist don't recall or unfamiliar with.
    In the dialog, we will update the following field:
      File Path - Where .blend is located
      Blender Version - last version .blend was saved in
      Output destination - output camera destination extracted. This can be changed.
      Timeline (Start/End) - active scene begin and end animation timeline.
  */
  function showDialog(file_path: string): void {
    invoke("import_blend", { path: file_path }).then(ctx => {
      if (ctx == null) {
        // I can't imagine how this would be null?
        console.log("import_blend received null?", ctx, file_path);
        return;
      }

      const data: BlendInfo = JSON.parse(ctx as string);      
      setBlendInfo(data);
      dialogRef.current?.showModal();
    });
  }

  function onJobSelected(job: RenderJobProps): void {
    setSelectedJob(job);
  }

  //#endregion

  return (
    <div className="content">

      <dialog ref={dialogRef}>
        <JobDialog info={blendInfo} 
                  versions={props.versions} 
                  onJobCreated={props.onJobCreated} 
                  onClose={() => dialogRef.current?.close()} 
        />
      </dialog>
            
      <h2>Remote Jobs</h2>
      
      {/* How can I enable hotkey function for html code? */}
      <button onClick={onImportClick}>
        Import
      </button>
      
      {/* Collection of active job list 
        // change this to point to sql table instead?
      */}
      <div className="group">
        {props.jobs.map((job) => RenderJob(job, onJobSelected))}
      </div>

      <JobDetail job={selectedJob}/>

    </div>
  );
}
