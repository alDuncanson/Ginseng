export type TransferId = string;
export type FileId = string;

export type TransferType = "upload" | "download";

export type TransferStage =
	| "initializing"
	| "connecting"
	| "transferring"
	| "finalizing"
	| "completed"
	| "failed"
	| "cancelled";

export type FileStatus = "pending" | "transferring" | "completed" | "failed" | "skipped";

export interface FileProgress {
	fileId: FileId;
	name: string;
	relativePath: string;
	totalBytes: number;
	transferredBytes: number;
	status: FileStatus;
	transferRate?: number;
	error?: string;
}

export interface TransferProgress {
	transferId: TransferId;
	transferType: TransferType;
	stage: TransferStage;
	totalFiles: number;
	completedFiles: number;
	failedFiles: number;
	totalBytes: number;
	transferredBytes: number;
	transferRate?: number;
	startTime: number;
	etaSeconds?: number;
	files: FileProgress[];
	error?: string;
}

export type ProgressEvent =
	| { event: "transferStarted"; data: { transfer: TransferProgress } }
	| { event: "transferProgress"; data: { transfer: TransferProgress } }
	| {
			event: "fileProgress";
			data: { transferId: TransferId; file: FileProgress };
	  }
	| {
			event: "stageChanged";
			data: { transferId: TransferId; stage: TransferStage; message?: string };
	  }
	| { event: "transferCompleted"; data: { transfer: TransferProgress } }
	| {
			event: "transferFailed";
			data: { transfer: TransferProgress; error: string };
	  };

export const formatBytes = (bytes: number): string => {
	const units = ["B", "KB", "MB", "GB", "TB"];
	if (bytes === 0) return "0 B";

	let size = bytes;
	let unitIndex = 0;

	while (size >= 1024 && unitIndex < units.length - 1) {
		size /= 1024;
		unitIndex++;
	}

	return `${size.toFixed(2)} ${units[unitIndex]}`;
};

export const formatDuration = (seconds: number): string => {
	const hours = Math.floor(seconds / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	const secs = seconds % 60;

	if (hours > 0) return `${hours}h ${minutes}m ${secs}s`;
	if (minutes > 0) return `${minutes}m ${secs}s`;
	return `${secs}s`;
};

export const calculateProgress = (transferred: number, total: number): number => {
	if (total === 0) return 0;
	return Math.round((transferred / total) * 100);
};
