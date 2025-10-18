import "./App.css";
import { Toaster } from "@/components/ui/sonner";
import { FileTransfer } from "@/components/FileTransfer";

function App() {
	return (
		<div className="min-h-screen bg-background">
			<FileTransfer />
			<Toaster />
		</div>
	);
}

export default App;
