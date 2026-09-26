/**
 * jsdom gaps the app relies on: matchMedia (theme), dialog.showModal (confirmations), scrolling
 * and object URLs (export). Each stub is the smallest thing that behaves like a browser.
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
