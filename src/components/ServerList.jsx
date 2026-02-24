import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";

function ServerList({ onServersChange }) {
  const [servers, setServers] = useState([]);
  const [loading, setLoading] = useState(true);

  const scan = async () => {
    setLoading(true);
    try {
      const result = await invoke("scan_servers");
      setServers(result);
    } catch (error) {
      console.error("Failed to scan servers:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    onServersChange?.(servers.length, scan);
  }, [servers.length]);

  useEffect(() => {
    scan();

    let unlisten;

    const setupListener = async () => {
      unlisten = await getCurrentWindow().onFocusChanged(
        ({ payload: focused }) => {
          if (focused) {
            scan();
          }
        }
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const handleVisit = async (url) => {
    try {
      await openUrl(url);
    } catch (error) {
      console.error("Failed to open URL:", error);
    }
  };

  const handleStop = async (pid) => {
    try {
      await invoke("kill_server", { pid });
      // Trigger a scan immediately to update the list
      setTimeout(scan, 500);
    } catch (error) {
      console.error("Failed to stop server:", error);
    }
  };

  const handleStart = async (server) => {
    if (!server.path || !server.command) return;
    try {
      await invoke("start_server", {
        cwd: server.path,
        command: server.command,
      });
      // Give it a moment to start before scanning. Scan twice, just in case
      // server starts after first scan.
      setTimeout(scan, 1000);
      setTimeout(scan, 4000);
    } catch (error) {
      console.error("Failed to start server:", error);
    }
  };

  if (loading && servers.length === 0) {
    return <div className="loading">Scanning...</div>;
  }

  if (servers.length === 0) {
    return (
      <div className="empty-state">
        <p>No servers detected.</p>
        <button onClick={scan} className="refresh-btn">
          Refresh
        </button>
      </div>
    );
  }

  return (
    <div className="server-list">
      {servers.map((server) => (
        <div
          key={server.path || server.port}
          className={`server-item ${!server.active ? "stopped" : ""}`}
        >
          <div className="server-info">
            <span className={`server-port ${server.active ? "running" : ""}`}>
              {server.active ? `:${server.port}` : ":oxox"}
            </span>
            <div className="server-details">
              <span className="server-title" title={server.title}>
                {server.title}
              </span>
              {server.path && (
                <span className="server-path" title={server.path}>
                  {server.path}
                </span>
              )}
            </div>
          </div>
          <div className="server-actions">
            {server.active ? (
              <>
                {server.pid && (
                  <button
                    className="stop-btn"
                    onClick={() => handleStop(server.pid)}
                    title="Stop Server"
                  >
                    Stop
                  </button>
                )}
                <button
                  className="visit-btn"
                  onClick={() => handleVisit(server.url)}
                  title="Open in Browser"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                    <polyline points="15 3 21 3 21 9"></polyline>
                    <line x1="10" y1="14" x2="21" y2="3"></line>
                  </svg>
                </button>
              </>
            ) : (
              <button
                className="start-btn"
                onClick={() => handleStart(server)}
                title="Start Server"
                disabled={!server.command}
              >
                Start
              </button>
            )}
            {server.path && (
              <button
                className="visit-btn"
                onClick={() => revealItemInDir(server.path)}
                title="Open in Finder"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                </svg>
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

export default ServerList;
