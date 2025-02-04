import "./styles.css";
import "./App.css";
import "./styles.css";
import * as GoIcons from "react-icons/go";
import * as Hi2Icon from "react-icons/hi2";

<div>
    <div className={"sidebar"}>
      <nav >
        <ul className="nav-menu-items">
          <li key="manager" className={"nav-bar"} hx-get="./pages/remote_render" hx-target="#workplace">
              <GoIcons.GoProject></GoIcons.GoProject>
              <span>Remote Render</span>
          </li>
          <li key="setting" className={"nav-bar"} hx-get="./pages/setting" hx-target="#workplace">
            <span><Hi2Icon.HiOutlineCog8Tooth></Hi2Icon.HiOutlineCog8Tooth></span>
            <span>Setting</span>
          </li>
        </ul>
      </nav>
      {/* {NodeWindow()} */}
    </div>

    <main id="workplace"></main>
</div>
