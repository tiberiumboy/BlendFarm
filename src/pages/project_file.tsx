export interface ProjectFileProps {
  id?: string;
  title: string;
  tmp?: string;
  edit?: Function;
}

// todo: expose function controls here. props event handler?
export default function ProjectFile(props: ProjectFileProps, key: Number = 0) {
  const handleClick = (e: any) => {
    if (props.edit) {
      props.edit();
    }
    alert("Editing!");
    return e;
  };

  return (
    <div className="item" key={key} id={props.id} onClick={handleClick}>
      {props.title}
    </div>
  );
}
