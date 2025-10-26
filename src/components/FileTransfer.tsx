import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Copy, Download, File, Folder, Files, Send, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";

interface FileInfo {
  name: string;
  relative_path: string;
  size: number;
}

interface ShareMetadata {
  files: FileInfo[];
  share_type: "SingleFile" | "MultipleFiles" | { Directory: { name: string } };
  total_size: number;
}

interface DownloadResult {
  metadata: ShareMetadata;
  download_path: string;
}

export function FileTransfer() {
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [ticket, setTicket] = useState<string>("");
  const [sendLoading, setSendLoading] = useState(false);

  const [receiveTicket, setReceiveTicket] = useState<string>("");
  const [receiveLoading, setReceiveLoading] = useState(false);
  const [lastDownload, setLastDownload] = useState<DownloadResult | null>(null);

  const selectFiles = async () => {
    try {
      const files = await open({
        multiple: true,
        directory: false,
      });
      if (files) {
        const fileArray = Array.isArray(files) ? files : [files];
        setSelectedPaths(fileArray as string[]);
        setTicket("");
      }
    } catch {
      toast.error("Failed to select files");
    }
  };

  const selectFolder = async () => {
    try {
      const folder = await open({
        multiple: false,
        directory: true,
      });
      if (folder) {
        setSelectedPaths([folder as string]);
        setTicket("");
      }
    } catch {
      toast.error("Failed to select folder");
    }
  };

  const removeFile = (index: number) => {
    const newPaths = selectedPaths.filter((_, i) => i !== index);
    setSelectedPaths(newPaths);
    if (newPaths.length === 0) {
      setTicket("");
    }
  };

  const sendFiles = async () => {
    if (selectedPaths.length === 0) {
      toast.error("Please select files or a folder first");
      return;
    }

    setSendLoading(true);
    try {
      const generatedTicket = await invoke<string>("share_files", {
        paths: selectedPaths,
      });
      setTicket(generatedTicket);
      toast.success("Share ticket generated!");
    } catch (error) {
      toast.error(`Failed to share files: ${error}`);
    } finally {
      setSendLoading(false);
    }
  };

  const copyTicket = async () => {
    try {
      await navigator.clipboard.writeText(ticket);
      toast.success("Ticket copied to clipboard");
    } catch {
      toast.error("Failed to copy ticket");
    }
  };

  const receiveFiles = async () => {
    if (!receiveTicket) {
      toast.error("Please enter a ticket");
      return;
    }

    setReceiveLoading(true);
    try {
      const result = await invoke<DownloadResult>("download_files", {
        ticket: receiveTicket,
      });
      setLastDownload(result);
      toast.success("Files downloaded successfully!");
      setReceiveTicket("");
    } catch (error) {
      toast.error(`Failed to download files: ${error}`);
    } finally {
      setReceiveLoading(false);
    }
  };

  const getFileName = (path: string) => {
    return path.split("/").pop() || path.split("\\").pop() || path;
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const getShareTypeDisplay = (shareType: ShareMetadata["share_type"]) => {
    if (shareType === "SingleFile") return "Single File";
    if (shareType === "MultipleFiles") return "Multiple Files";
    if (typeof shareType === "object" && "Directory" in shareType) {
      return `Directory: ${shareType.Directory.name}`;
    }
    return "Unknown";
  };

  const getSelectionSummary = () => {
    if (selectedPaths.length === 0) return "No files selected";
    if (selectedPaths.length === 1) {
      const path = selectedPaths[0];
      return getFileName(path);
    }
    return `${selectedPaths.length} items selected`;
  };

  return (
    <div className="max-w-2xl mx-auto p-6">
      <div className="text-center mb-8">
        <h1 className="text-3xl font-bold mb-2">Ginseng</h1>
        <p className="text-muted-foreground">
          Secure peer-to-peer file sharing
        </p>
      </div>

      <Tabs defaultValue="send" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="send">Send</TabsTrigger>
          <TabsTrigger value="receive">Receive</TabsTrigger>
        </TabsList>

        <TabsContent value="send">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Send className="h-5 w-5" />
                Share Files
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Select Files or Folder</Label>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    onClick={selectFiles}
                    className="flex-1 justify-start"
                  >
                    <Files className="h-4 w-4 mr-2" />
                    Select Files
                  </Button>
                  <Button
                    variant="outline"
                    onClick={selectFolder}
                    className="flex-1 justify-start"
                  >
                    <Folder className="h-4 w-4 mr-2" />
                    Select Folder
                  </Button>
                </div>
              </div>

              {selectedPaths.length > 0 && (
                <div className="space-y-2">
                  <Label>Selected Items ({selectedPaths.length})</Label>
                  <div className="max-h-40 overflow-y-auto space-y-1 p-2 border rounded">
                    {selectedPaths.map((path, index) => (
                      <div
                        key={index}
                        className="flex items-center justify-between p-2 bg-muted rounded text-sm"
                      >
                        <div className="flex items-center gap-2 flex-1 min-w-0">
                          <File className="h-4 w-4 flex-shrink-0" />
                          <span className="truncate" title={path}>
                            {getFileName(path)}
                          </span>
                        </div>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => removeFile(index)}
                          className="h-6 w-6 p-0 flex-shrink-0"
                        >
                          <X className="h-3 w-3" />
                        </Button>
                      </div>
                    ))}
                  </div>
                  <p className="text-sm text-muted-foreground">
                    {getSelectionSummary()}
                  </p>
                </div>
              )}

              <Button
                onClick={sendFiles}
                disabled={selectedPaths.length === 0 || sendLoading}
                className="w-full"
              >
                {sendLoading ? "Generating..." : "Generate Share Ticket"}
              </Button>

              {ticket && (
                <div className="space-y-2">
                  <Label>Share Ticket</Label>
                  <div className="flex gap-2">
                    <Input
                      value={ticket}
                      readOnly
                      className="font-mono text-xs"
                    />
                    <Button variant="outline" size="icon" onClick={copyTicket}>
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Copy this ticket and send it to the receiver
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="receive">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Download className="h-5 w-5" />
                Receive Files
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Ticket</Label>
                <Input
                  placeholder="Paste the share ticket here..."
                  value={receiveTicket}
                  onChange={(e) => setReceiveTicket(e.target.value)}
                  className="font-mono text-xs"
                />
                <p className="text-sm text-muted-foreground">
                  Files will be downloaded to your Downloads folder
                  automatically
                </p>
              </div>

              <Button
                onClick={receiveFiles}
                disabled={!receiveTicket || receiveLoading}
                className="w-full"
              >
                {receiveLoading ? "Downloading..." : "Download Files"}
              </Button>

              {lastDownload && (
                <div className="space-y-3 p-4 border rounded bg-muted/50">
                  <Label>Last Download</Label>
                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">Type:</span>
                      <Badge variant="secondary">
                        {getShareTypeDisplay(lastDownload.metadata.share_type)}
                      </Badge>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">Files:</span>
                      <span className="text-sm">
                        {lastDownload.metadata.files.length}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">Total Size:</span>
                      <span className="text-sm">
                        {formatFileSize(lastDownload.metadata.total_size)}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">Location:</span>
                      <span
                        className="text-sm truncate ml-2"
                        title={lastDownload.download_path}
                      >
                        {lastDownload.download_path}
                      </span>
                    </div>
                  </div>

                  {lastDownload.metadata.files.length > 0 && (
                    <div className="space-y-1">
                      <span className="text-sm font-medium">Files:</span>
                      <div className="max-h-32 overflow-y-auto space-y-1">
                        {lastDownload.metadata.files.map((file, index) => (
                          <div
                            key={index}
                            className="flex items-center justify-between text-xs p-1 bg-background rounded"
                          >
                            <span
                              className="truncate"
                              title={file.relative_path}
                            >
                              {file.relative_path}
                            </span>
                            <span className="text-muted-foreground ml-2">
                              {formatFileSize(file.size)}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
