export function saveBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  // 延迟释放，避免部分浏览器在点击后立即 revoke 导致下载中断。
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}
