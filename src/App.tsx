import "./App.css";
import { FileTransfer } from "@/components/FileTransfer";
import { Toaster } from "@/components/ui/sonner";

function App() {
	return (
		<div className="min-h-screen bg-background">
			<FileTransfer />
			<Toaster />
		</div>
	);
}

export default App;
