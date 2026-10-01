/**
 * jsdom gaps the app relies on: matchMedia (theme), dialog.showModal (confirmations), scrolling,
 * object URLs (export) and Blob.text (reading a chosen plan file). Each stub is the smallest thing
 * that behaves like a browser.
 */
if (!window.matchMedia) {
  window.matchMedia = (query: string): MediaQueryList =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList;
}

if (typeof HTMLDialogElement !== 'undefined') {
  const proto = HTMLDialogElement.prototype;
  proto.showModal ??= function (this: HTMLDialogElement) {
    this.setAttribute('open', '');
  };
  proto.close ??= function (this: HTMLDialogElement) {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
}

window.scrollTo = () => {};
Element.prototype.scrollIntoView ??= function () {};
URL.createObjectURL ??= () => 'blob:test';
URL.revokeObjectURL ??= () => {};

// Every browser reads a chosen file with File.text(); jsdom has FileReader but not Blob.text.
if (typeof Blob !== 'undefined' && !Blob.prototype.text) {
  Blob.prototype.text = function (this: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(reader.error);
      reader.readAsText(this);
    });
  };
}
