import { useState } from "react";

function ConnectionContainer() {
  const [list, setList] = useState([]);

  function drawConnectionItem(item: RenderNode) {}

  function loadList() {
    return list.map(drawConnectionItem);
  }

  return (
    <div className="group" id="connection-list">
      {loadList()}
    </div>
  );
}

export default ConnectionContainer;
