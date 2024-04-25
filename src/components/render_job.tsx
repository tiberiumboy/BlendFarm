import { ProjectFileProps } from "../pages/project_file";

export interface RenderJobProps {
  id: string;
  project_file: ProjectFileProps;
}

export default function RenderJob(job: RenderJobProps) {
  return (
    <div>
      <table>
        <tr>
          <td>{job.project_file.file_name}</td>
          {/* TODO: Add controls to remove job from list */}
          <td></td>
        </tr>
      </table>
    </div>
  );
}
