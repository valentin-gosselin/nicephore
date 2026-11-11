<script lang="ts">
  /**
   * Video player with the controls of the Immich web app (media-chrome):
   * play, time, volume opening on hover, speed, full screen and a thin
   * progress bar, instead of the controls of the system's web engine.
   */
  import { Icon } from '@immich/ui';
  import {
    mdiChevronLeft,
    mdiChevronRight,
    mdiFullscreen,
    mdiFullscreenExit,
    mdiPause,
    mdiPlay,
    mdiVolumeHigh,
    mdiVolumeLow,
    mdiVolumeMedium,
    mdiVolumeMute,
  } from '@mdi/js';
  import { langue, t } from './i18n.svelte';
  import 'media-chrome/media-control-bar';
  import 'media-chrome/media-controller';
  import 'media-chrome/media-fullscreen-button';
  import 'media-chrome/media-mute-button';
  import 'media-chrome/media-play-button';
  import 'media-chrome/media-time-display';
  import 'media-chrome/media-time-range';
  import 'media-chrome/media-volume-range';
  import 'media-chrome/menu/media-playback-rate-menu';
  import 'media-chrome/menu/media-settings-menu';
  import 'media-chrome/menu/media-settings-menu-button';
  import 'media-chrome/menu/media-settings-menu-item';

  interface Props {
    src: string;
    /** Classes of the video element itself (size, fit). */
    classeVideo?: string;
    autoplay?: boolean;
    onloadedmetadata?: (video: HTMLVideoElement) => void;
    onerror?: () => void;
    onended?: () => void;
  }

  let { src, classeVideo = '', autoplay = true, onloadedmetadata, onerror, onended }: Props = $props();
</script>

<!-- Left and right arrows stay with the viewer, to go to the next file. -->
<media-controller dir="ltr" lang={langue.code} hotkeys="noarrowleft noarrowright" class="lecteur dark">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    slot="media"
    class={classeVideo}
    {src}
    {autoplay}
    playsinline
    disablepictureinpicture
    onloadedmetadata={(e) => onloadedmetadata?.(e.currentTarget)}
    onerror={() => onerror?.()}
    onended={() => onended?.()}
  ></video>

  <media-settings-menu hidden anchor="auto" class="menu-reglages">
    <media-settings-menu-item class="element-menu">
      {t('Vitesse')}
      <Icon slot="suffix" icon={mdiChevronRight} class="m-2" />
      <media-playback-rate-menu slot="submenu" hidden rates="0.5 1 1.5 2">
        <Icon slot="back-icon" icon={mdiChevronLeft} class="m-2" />
        <span slot="title">{t('Vitesse')}</span>
      </media-playback-rate-menu>
    </media-settings-menu-item>
  </media-settings-menu>

  <div class="relative isolate flex h-32 w-full flex-col justify-end px-4">
    <div class="degrade pointer-events-none absolute inset-x-0 bottom-0 -z-10 h-48"></div>
    <media-control-bar class="flex h-10 w-full gap-2">
      <media-play-button class="shrink-0 rounded-full p-2 outline-none">
        <Icon slot="play" icon={mdiPlay} />
        <Icon slot="pause" icon={mdiPause} />
      </media-play-button>
      <media-time-display showduration class="rounded-lg p-2 outline-none"></media-time-display>
      <span class="grow"></span>
      <div class="volume shrink-0 rounded-full">
        <media-volume-range class="h-full bg-none outline-none"></media-volume-range>
        <media-mute-button class="bg-none p-2 outline-none">
          <Icon slot="off" icon={mdiVolumeMute} />
          <Icon slot="low" icon={mdiVolumeLow} />
          <Icon slot="medium" icon={mdiVolumeMedium} />
          <Icon slot="high" icon={mdiVolumeHigh} />
        </media-mute-button>
      </div>
      <media-fullscreen-button class="shrink-0 rounded-full p-2 outline-none">
        <Icon slot="enter" icon={mdiFullscreen} />
        <Icon slot="exit" icon={mdiFullscreenExit} />
      </media-fullscreen-button>
      <media-settings-menu-button class="shrink-0 rounded-full p-2 outline-none"></media-settings-menu-button>
    </media-control-bar>
    <media-time-range class="h-8 w-full rounded-lg px-2 pb-3 outline-none"></media-time-range>
  </div>
</media-controller>

<style>
  media-controller {
    width: 100%;
    height: 100%;
    background: transparent;
    --media-background-color: transparent;
    --media-control-background: none;
    --media-control-hover-background: var(--immich-ui-light-100);
    --media-focus-box-shadow: 0 0 0 2px var(--immich-ui-dark);
    --media-font-family: inherit;
    --media-font-size: 1rem;
    --media-font-weight: 500;
    --media-menu-border-radius: 0.75rem;
    --media-menu-gap: 0.25rem;
    --media-menu-item-hover-background: var(--immich-ui-light-200);
    --media-menu-item-icon-height: 1em;
    --media-menu-item-indicator-height: 1em;
    --media-primary-color: var(--immich-ui-dark);
    --media-secondary-color: transparent;
    --media-time-range-buffered-color: var(--immich-ui-dark-400);
    --media-time-range-hover-bottom: 0;
    --media-time-range-hover-height: 100%;
    --media-range-thumb-box-shadow: none;
    --media-range-thumb-opacity: 0;
    --media-range-thumb-transition: opacity 0.15s ease;
    --media-range-track-border-radius: 2px;
    --media-range-track-height: 3.5px;
    --media-range-padding: 0;
    --media-settings-menu-background: var(--immich-ui-light-100);
    --media-text-content-height: 1.5rem;
    --media-tooltip-arrow-display: none;
    --media-tooltip-border-radius: 0.5rem;
    --media-tooltip-background-color: var(--immich-ui-light-200);
    --media-tooltip-distance: 8px;
    --media-tooltip-padding: 0.5rem 0.875rem;
  }

  .degrade {
    background: linear-gradient(to top, rgb(0 0 0 / 0.8), rgb(0 0 0 / 0));
  }

  media-time-display {
    font-variant-numeric: tabular-nums;
  }

  .menu-reglages {
    min-width: 16rem;
    border: 1px solid var(--immich-ui-light-300);
    border-radius: 0.75rem;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.1);
  }

  .element-menu {
    margin: 0 0.25rem;
    border-radius: 0.5rem;
    padding: 0.25rem 0.25rem 0.25rem 0.5rem;
  }

  media-time-range,
  media-volume-range {
    --media-control-hover-background: none;
  }

  media-time-range:hover,
  media-volume-range:hover {
    --media-range-thumb-opacity: 1;
  }

  :global(media-controller.lecteur *::part(tooltip)) {
    --media-font-size: 0.75rem;
    --media-text-content-height: 1rem;
    color: white;
  }

  :global(media-controller.lecteur[mediavolumeunavailable]) {
    --media-volume-range-display: none;
  }

  .volume {
    --media-control-hover-background: none;
    transition: background-color 0.4s;
  }

  .volume:hover {
    background-color: var(--immich-ui-light-100);
  }

  media-volume-range {
    padding: 0;
    margin: 0;
    width: 0;
    overflow: hidden;
    transition: width 0.4s ease-out;
  }

  /* The volume opens on hover or focus, as in Immich. */
  .volume:hover > media-volume-range,
  .volume:focus-within > media-volume-range {
    padding: 0 0.5rem;
    margin-left: 0.5rem;
    width: 70px;
  }
</style>
