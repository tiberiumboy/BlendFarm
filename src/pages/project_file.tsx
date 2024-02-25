
type ProjectFileProps = {
    id?: string,
    title: string,
    src: string,
    tmp?: string,
}

export default function ProjectFile(props: ProjectFileProps) {
    return (
        <div className="item" id={props.id}>
            {props.title}
        </div>
    )
}
