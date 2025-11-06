import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { Upload } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

interface DropZoneProps {
	onPathsSelected: (paths: string[]) => void;
	disabled?: boolean;
}

export function DropZone({ onPathsSelected, disabled = false }: DropZoneProps) {
	const [isDragging, setIsDragging] = useState(false);
	const dropZoneRef = useRef<HTMLDivElement>(null);
	const isMouseOverRef = useRef(false);

	useEffect(() => {
		const setupListeners = async () => {
			const webview = getCurrentWebview();
			const unlisten = await webview.onDragDropEvent((event) => {
				if (event.payload.type === "enter") {
					if (!disabled) {
						setIsDragging(true);
					}
				} else if (event.payload.type === "leave") {
					setIsDragging(false);
				} else if (event.payload.type === "drop") {
					setIsDragging(false);
					if (!disabled && event.payload.paths.length > 0) {
						onPathsSelected(event.payload.paths);
					}
				}
			});

			return unlisten;
		};

		let cleanup: (() => void) | undefined;
		setupListeners().then((fn) => {
			cleanup = fn;
		});

		return () => {
			cleanup?.();
		};
	}, [disabled, onPathsSelected]);

	const handleClick = async () => {
		if (disabled) return;

		try {
			const result = await open({
				multiple: true,
				directory: false,
			});

			if (result) {
				const paths = Array.isArray(result) ? result : [result];
				onPathsSelected(paths as string[]);
			}
		} catch {
			toast.error("Failed to select files");
		}
	};

	const handleFolderClick = async () => {
		if (disabled) return;

		try {
			const result = await open({
				multiple: false,
				directory: true,
			});

			if (result) {
				onPathsSelected([result as string]);
			}
		} catch {
			toast.error("Failed to select folder");
		}
	};

	return (
		<div
			ref={dropZoneRef}
			onMouseEnter={() => {
				isMouseOverRef.current = true;
			}}
			onMouseLeave={() => {
				isMouseOverRef.current = false;
			}}
			className={`
				relative border-2 border-dashed rounded-lg p-12 w-full
				transition-all duration-200
				${isDragging ? "border-primary bg-primary/5 scale-[0.99]" : "border-foreground/20"}
				${disabled ? "opacity-50 cursor-not-allowed" : "hover:border-foreground/40"}
			`}
		>
			<div className="flex flex-col items-center gap-6 text-center">
				<Upload
					className={`h-12 w-12 transition-transform ${isDragging ? "scale-110 text-primary" : "text-muted-foreground"}`}
				/>
				<div className="space-y-2">
					<p className="text-sm font-normal">
						{isDragging ? "Drop files or folders here" : "Drag & drop files or folders"}
					</p>
					<p className="text-xs text-muted-foreground">or click to browse</p>
				</div>

				<div className="flex gap-2">
					<button
						type="button"
						onClick={handleClick}
						disabled={disabled}
						className="px-4 py-2 text-xs border border-foreground/20 rounded hover:bg-foreground/5 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
					>
						Select Files
					</button>
					<button
						type="button"
						onClick={handleFolderClick}
						disabled={disabled}
						className="px-4 py-2 text-xs border border-foreground/20 rounded hover:bg-foreground/5 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
					>
						Select Folder
					</button>
				</div>
			</div>
		</div>
	);
}
