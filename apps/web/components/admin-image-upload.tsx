"use client";

import { useRef, useState, type ChangeEvent } from "react";
import Cropper, { type Area } from "react-easy-crop";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";

const ACCEPTED_TYPES = ["image/jpeg", "image/png", "image/webp"];
// Exported square size, not the original resolution — keeps the uploaded
// file small by construction, on top of the backend's hard 5 MiB cap.
const CROP_OUTPUT_SIZE = 800;

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("failed to load the selected image"));
    image.src = src;
  });
}

async function cropToBlob(imageSrc: string, area: Area, mimeType: string): Promise<Blob> {
  const image = await loadImage(imageSrc);
  const canvas = document.createElement("canvas");
  canvas.width = CROP_OUTPUT_SIZE;
  canvas.height = CROP_OUTPUT_SIZE;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas is not supported");

  ctx.drawImage(image, area.x, area.y, area.width, area.height, 0, 0, CROP_OUTPUT_SIZE, CROP_OUTPUT_SIZE);

  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error("failed to export the crop"))), mimeType);
  });
}

/**
 * requirement.md Decision 1 (upload via this API's own proxy, never a
 * direct-to-MinIO or pasted URL) and Decision 14's addendum (crop to a fixed
 * 1:1 square client-side before upload). Reports the final MinIO-backed URL
 * via `onUploaded` — the parent form holds that value, not this component.
 */
export function AdminImageUpload({
  currentImageUrl,
  onUploaded,
}: {
  currentImageUrl: string | null;
  onUploaded: (url: string) => void;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [localFile, setLocalFile] = useState<{ objectUrl: string; mimeType: string } | null>(null);
  const [crop, setCrop] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [croppedAreaPixels, setCroppedAreaPixels] = useState<Area | null>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [uploadedUrl, setUploadedUrl] = useState<string | null>(currentImageUrl);

  function handleFileSelected(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;

    if (!ACCEPTED_TYPES.includes(file.type)) {
      setError("Please choose a JPEG, PNG, or WebP image.");
      return;
    }

    setError(null);
    setCrop({ x: 0, y: 0 });
    setZoom(1);
    setCroppedAreaPixels(null);
    setLocalFile({ objectUrl: URL.createObjectURL(file), mimeType: file.type });
  }

  function resetFileInput() {
    if (fileInputRef.current) fileInputRef.current.value = "";
  }

  function handleCancelCrop() {
    if (localFile) URL.revokeObjectURL(localFile.objectUrl);
    setLocalFile(null);
    setError(null);
    resetFileInput();
  }

  async function handleConfirmCrop() {
    if (!localFile || !croppedAreaPixels) return;

    setUploading(true);
    setError(null);

    try {
      const blob = await cropToBlob(localFile.objectUrl, croppedAreaPixels, localFile.mimeType);
      const formData = new FormData();
      formData.append("file", blob, "photo");

      const response = await fetch("/api/admin/media", { method: "POST", body: formData });

      if (!response.ok) {
        // A `413` from the API's DefaultBodyLimit layer comes back as plain
        // text, not JSON (requirement.md open risk 4) — never assume the
        // body parses.
        const contentType = response.headers.get("content-type") ?? "";
        setError(
          contentType.includes("application/json")
            ? "That photo couldn't be uploaded — check its type and size."
            : "Upload failed — try a smaller image.",
        );
        return;
      }

      const body = (await response.json()) as { url: string };
      setUploadedUrl(body.url);
      onUploaded(body.url);
      URL.revokeObjectURL(localFile.objectUrl);
      setLocalFile(null);
      resetFileInput();
    } catch {
      setError("Upload failed — try a smaller image.");
    } finally {
      setUploading(false);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <div>
        <span className="block text-sm font-semibold">Photo (required)</span>
        <p className="m-0 mt-0.5 text-sm text-text-muted">
          JPEG, PNG, or WebP — you&apos;ll be able to crop it to a square before saving.
        </p>
      </div>

      {localFile ? (
        <div className="flex flex-col gap-3">
          <div className="relative h-[280px] w-full max-w-[360px] overflow-hidden rounded-card bg-background-soft">
            <Cropper
              image={localFile.objectUrl}
              crop={crop}
              zoom={zoom}
              aspect={1}
              onCropChange={setCrop}
              onZoomChange={setZoom}
              onCropComplete={(_area, areaPixels) => setCroppedAreaPixels(areaPixels)}
            />
          </div>
          <div className="max-w-[360px]">
            <label htmlFor="crop-zoom" className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-text-muted">
              Zoom
            </label>
            <input
              id="crop-zoom"
              type="range"
              min={1}
              max={3}
              step={0.1}
              value={zoom}
              onChange={(event) => setZoom(Number(event.target.value))}
              className="w-full"
            />
          </div>
          <div className="flex gap-3">
            <Button type="button" onClick={handleConfirmCrop} disabled={uploading}>
              {uploading ? "Uploading…" : "Use this photo"}
            </Button>
            <Button type="button" variant="secondary" onClick={handleCancelCrop} disabled={uploading}>
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex items-center gap-4">
          {uploadedUrl && (
            // eslint-disable-next-line @next/next/no-img-element -- a freshly uploaded photo's URL isn't guaranteed to be a configured next/image remote pattern until this page is reloaded.
            <img src={uploadedUrl} alt="" className="h-20 w-20 rounded-card object-cover" />
          )}
          <label
            htmlFor="menu-item-photo"
            className="flex max-w-[360px] flex-1 cursor-pointer flex-col items-center gap-2 rounded-card border-2 border-dashed border-text-muted/40 bg-background-soft px-4 py-6 text-center transition-colors hover:border-primary"
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
              className="h-6 w-6 text-text-muted"
            >
              <path d="M12 16V4" />
              <path d="M7 9l5-5 5 5" />
              <path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3" />
            </svg>
            <span className="text-sm font-semibold">
              {uploadedUrl ? "Click to replace this photo" : "Click to upload a photo"}
            </span>
            <input
              ref={fileInputRef}
              id="menu-item-photo"
              type="file"
              accept="image/jpeg,image/png,image/webp"
              onChange={handleFileSelected}
              className="sr-only"
            />
          </label>
        </div>
      )}

      {error && <Notice>{error}</Notice>}
    </div>
  );
}
