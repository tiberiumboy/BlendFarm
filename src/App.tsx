// import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import "./styles.css";
import Sidebar from "./components/side_bar";
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Setting from "./pages/setting";
import LiveView from "./pages/live_view";
import RemoteRender from "./pages/remote_render";

function App() {
  return (
    <div>
      <Router>
        <Sidebar />
        <Routes>
          <Route path='/' Component={RemoteRender} />
          <Route path='/remote_render' Component={RemoteRender} />
          <Route path='/setting' Component={Setting} />
          <Route path='/liveview' Component={LiveView} />
        </Routes>
      </Router>
    </div>
  );
}

export default App;
