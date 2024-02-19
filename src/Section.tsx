type SectionProps = {
    displayName: string;
    page: string;
    isActive?: boolean;
    onClick: (url: string) => void;
}

function Section(props: SectionProps) {

    function handleClick(e: any) {
        e.preventDefault();
        props.onClick(props.page)
        return false;
    }

    return (
        
    );
}

export default Section;