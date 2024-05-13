import { ProjectFileProps } from "../components/project_file";
import { CiCircleMore, CiTrash } from "react-icons/ci";
import { invoke, convertFileSrc } from "@tauri-apps/api/tauri";

export interface RenderJobProps {
  id: string;
  project_file: ProjectFileProps;
  src?: string;
  image_pic?: string;
  onDataChanged?: () => void;
}

export default function RenderJob(job: RenderJobProps) {
  const deleteJob = () =>
    invoke("delete_job", { targetJob: job }).then(job.onDataChanged);

  const moreAction = () => {
    // should probably provide some context menu?
    console.log("more action was pressed | TODO: add impl.");
  };

  const showCompletedImage = () => {
    if (job.image_pic != null) {
      return (
        <img
          style={{ height: "100%", width: "100%", objectFit: "contain" }}
          src={convertFileSrc(job.image_pic)}
        />
      );
    } else {
      return <div></div>;
    }
  };

  return (
    <div>
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>{job.project_file.file_name}</td>
            {/* <td>{job.project_file.src}</td> */}
            <td>
              <CiTrash onClick={deleteJob} />
            </td>
            <td>
              <CiCircleMore onClick={moreAction} />
            </td>
          </tr>
        </tbody>
      </table>
      {showCompletedImage()}
    </div>
  );
}
