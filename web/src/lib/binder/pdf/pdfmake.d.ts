/**
 * The part of pdfmake's browser build (`pdfmake/build/pdfmake.min.js`, MIT) the binder uses. The
 * build is one webpack bundle with no types of its own; these describe pdfmake 0.3's instance.
 */
declare module 'pdfmake/build/pdfmake.min.js' {
  export interface PdfOutput {
    getBuffer(): Promise<Uint8Array>;
    getBlob(): Promise<Blob>;
  }
  export interface PdfMakeInstance {
    createPdf(definition: object, options?: object): PdfOutput;
    /** Files by name: base64 text, as the font containers carry them. */
    addVirtualFileSystem(vfs: Record<string, string>): void;
    setFonts(fonts: Record<string, Record<'normal' | 'bold' | 'italics' | 'bolditalics', string>>): void;
    setUrlAccessPolicy(policy: (url: string) => boolean): void;
    /** Read by `createPdf` (the browser build has no setter); denies every local file path. */
    localAccessPolicy?: (path: string) => boolean;
  }
  const pdfMake: PdfMakeInstance;
  export default pdfMake;
}
