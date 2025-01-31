import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { RenderJobProps } from "./render_job";
import { BlendInfo } from "../pages/remote_render";

export default function JobDialog(props: { info: BlendInfo | null, versions: string[], onJobCreated: (job: RenderJobProps) => void, onClose: () => void }) {

    const [path, setPath] = useState<string>("");
    const [version, setVersion] = useState<string>("");
    const [startFrame, setStartFrame] = useState<number>(0);
    const [endFrame, setEndFrame] = useState<number>(1);
    const [output, setOutput] = useState<string>("");

    const dialogRef = useRef<HTMLDialogElement | null>(null);

    useEffect(() => {
      if ( !props.info ) return;
      setPath(props.info.path);
      setVersion(props.info.blend_version);
      setStartFrame(props.info.start);
      setEndFrame(props.info.end);
      setOutput(props.info.output);
    }, [props.info]);

    // TODO: find a way to make this more sense and pure function as possible.
    // see if I can just invoke a rust backend to handle file directory or file open instead?
    async function onDirectorySelect(e: any) {
      const filePath = await open({
        directory: true,
        multiple: false,
      });
      if (filePath != null) {
        // TODO: find a way to include this dash elsewhere
        e.target.value = filePath + "/";
      }
    }

    const handleSubmitJobForm = (e: React.FormEvent) => {
      e.preventDefault();
  
      // How do I structure this?
      const info = e.target as HTMLFormElement;
      // const selectedMode = info.modes.value;
      const output = info.output.value;
      const mode = {
        Animation: {
          start: Number(info.start.value),
          end: Number(info.end.value),
        },
      };
  
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
  
        let data: RenderJobProps = {
          current_frame: ctx.start,
          id: ctx.id,
          start_frame: ctx.start,
          end_frame: ctx.end,
          output: ctx.output,
          project_file: ctx.project_file,
          renders: [],
          version: ctx.blender_version,
        };
        props.onJobCreated(data);
      });
  
      props.onClose();
    }

    return (
        <div>
            <form method="dialog" onSubmit={handleSubmitJobForm}>
                <h1>Create new Render Job</h1>
                <label>Project File Path:</label>
                <input type="text" defaultValue={props.info?.path} placeholder="Project path" readOnly={true} />
            
                <br />
                <label>Blender Version:</label>
                <select defaultValue={props.info?.blend_version} onChange={(e) => setVersion(e.target.value)}>
                    {props.versions.map((item, index) => (
                    <option key={index} value={item}>{item}</option>
                    ))}
                </select>

                <div key="frameRangeEntry">
                    <label key="frameStartLabel" htmlFor="start">Start</label>
                    <input key="frameStartField" 
                          name="start" 
                          type="number" 
                          defaultValue={props.info?.start} 
                          onChange={(e) => setStartFrame(Number(e.target.value))}
                    />
                    <label key="frameEndLabel" htmlFor="end">End</label>
                    <input key="frameEndField" 
                          name="end" 
                          type="number" 
                          defaultValue={props.info?.end} 
                          onChange={(e) => setEndFrame(Number(e.target.value))} 
                    />
                </div>

                <label>Output destination:</label>
                <input
                    type="text"
                    placeholder="Output Path"
                    id="output"
                    name="output"
                    defaultValue={props.info?.output}
                    readOnly={true}
                    onChange={(e) => setOutput(e.target.value)}
                    onClick={onDirectorySelect}
                />
                <menu>
                    <button type="button" value="cancel" onClick={() => props.onClose()}>Cancel</button>
                    <button type="submit">Ok</button>
                </menu>
            </form>
        </div>
    );
}