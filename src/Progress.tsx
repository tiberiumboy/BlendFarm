type ProgressProps = {
    percentage: number;
}

function Progress(props: ProgressProps) {
    return (
        <div id="progressbar">
            <div>
                {/* style={width = { props.percentage }%} */}
                {props.percentage}%
            </div>
        </div >
    );
}

export default Progress;