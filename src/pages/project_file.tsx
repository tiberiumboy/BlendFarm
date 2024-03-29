export interface ProjectFileProps {
  id: string;
  src: string;
  edit?: Function;
}

// todo: expose function controls here. props event handler?
export default function ProjectFile(props: ProjectFileProps) {
  const handleClick = (e: any) => {
    e.preventDefault();
    if (props.edit) {
      props.edit();
    }
    return false;
  };

  return (
    <div className="item" key={props.id} id={props.id} onClick={handleClick}>
      {props.src}
    </div>
  );
}
