import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/globals.css";

// ============================================================================
// Error Boundary — catches rendering errors and shows fallback UI
// ============================================================================

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  ErrorBoundaryState
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("[ErrorBoundary] React render error:", error);
    console.error("[ErrorBoundary] Component stack:", errorInfo.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div
          style={{
            padding: "40px",
            fontFamily: "monospace",
            backgroundColor: "#1a1a2e",
            color: "#e0e0e0",
            minHeight: "100vh",
          }}
        >
          <h1 style={{ color: "#ff6b6b", fontSize: "24px" }}>
            ⚠ EK86317A Programmer — Render Error
          </h1>
          <p style={{ color: "#ffa07a", marginTop: "16px" }}>
            The application encountered an error during rendering.
          </p>
          <pre
            style={{
              marginTop: "16px",
              padding: "16px",
              backgroundColor: "#0d1117",
              borderRadius: "8px",
              overflow: "auto",
              maxHeight: "300px",
              fontSize: "13px",
              color: "#ff7b7b",
              border: "1px solid #333",
            }}
          >
            {this.state.error?.message}
            {"\n\n"}
            {this.state.error?.stack}
          </pre>
          <button
            onClick={() => {
              this.setState({ hasError: false, error: null });
            }}
            style={{
              marginTop: "20px",
              padding: "10px 24px",
              backgroundColor: "#4a90d9",
              color: "white",
              border: "none",
              borderRadius: "6px",
              cursor: "pointer",
              fontSize: "14px",
            }}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// ============================================================================
// App Entry Point
// ============================================================================

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
