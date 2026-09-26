<script lang="ts" module>
  export type IconName =
    | 'check'
    | 'circle'
    | 'info'
    | 'alert'
    | 'chevron-left'
    | 'chevron-right'
    | 'print'
    | 'download'
    | 'upload'
    | 'trash'
    | 'plus'
    | 'minus'
    | 'clock'
    | 'shield'
    | 'home'
    | 'menu'
    | 'close'
    | 'lock'
    | 'refresh'
    | 'calendar';

  const PATHS: Record<IconName, string> = {
    check: 'M5 12.5l4.5 4.5L19 7.5',
    circle: 'M12 3.5a8.5 8.5 0 1 0 0 17a8.5 8.5 0 1 0 0-17z',
    info: 'M12 3.5a8.5 8.5 0 1 0 0 17a8.5 8.5 0 1 0 0-17zM12 11v5.5M12 7.6v.4',
    alert: 'M12 4l9 16H3zM12 10v4.5M12 17.2v.3',
    'chevron-left': 'M14.5 6l-6 6l6 6',
    'chevron-right': 'M9.5 6l6 6l-6 6',
    print: 'M7 9V4h10v5M7 17H5a1 1 0 0 1-1-1v-5a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v5a1 1 0 0 1-1 1h-2M7 14h10v6H7z',
    download: 'M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19.5h14',
    upload: 'M12 15.5V4.5M7.5 9L12 4.5L16.5 9M5 19.5h14',
    trash: 'M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12M10.5 10.5v6M13.5 10.5v6',
    plus: 'M12 5v14M5 12h14',
    minus: 'M5 12h14',
    clock: 'M12 3.5a8.5 8.5 0 1 0 0 17a8.5 8.5 0 1 0 0-17zM12 7.5V12l3 2',
    shield: 'M12 3.5l7 3v5.5c0 4.2-3 7.4-7 8.5c-4-1.1-7-4.3-7-8.5V6.5z',
    home: 'M4 11l8-6.5l8 6.5M6.5 9.5V19h11V9.5',
    menu: 'M4.5 7h15M4.5 12h15M4.5 17h15',
    close: 'M6 6l12 12M18 6L6 18',
    lock: 'M7 11V8a5 5 0 0 1 10 0v3M5.5 11h13v9h-13z',
    refresh: 'M19 12a7 7 0 1 1-2.1-5M19 4.5V9h-4.5',
    calendar: 'M5 6.5h14V19H5zM5 10.5h14M9 4.5v4M15 4.5v4',
  };
</script>

<script lang="ts">
  let { name, size = '1.25em', label }: { name: IconName; size?: string; label?: string } = $props();
</script>

<svg
  class="icon"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  <path d={PATHS[name]} />
</svg>

<style>
  .icon {
    display: inline-block;
    flex: none;
    vertical-align: -0.2em;
  }
</style>
