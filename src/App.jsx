import "./App.css";
import ServerList from "./components/ServerList";
import icon from "./assets/icon-128x128.png";

function App() {
  return (
    <div className="container">
      <div className="main-panel">
        <main>
          <ServerList />
        </main>
        <header>
          <img src={icon} alt="Home127 Icon" className="app-icon" />
          <h1>Home127</h1>
        </header>
      </div>
    </div>
  );
}

export default App;
