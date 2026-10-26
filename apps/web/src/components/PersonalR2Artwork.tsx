import React from 'react';

/** Personal presentation only: never treat this illustration as reported paint/configuration. */
export function isPersonalR2Model(model: string | null | undefined): boolean {
  return /^R2S?$/i.test(model?.trim() ?? '');
}

/** One asset for Overview and Health so a visual update cannot leave them out of sync. */
export function PersonalR2Artwork({ className }: { className?: string }) {
  return (
    <img
      src="/r2-dashboard/r2-launch-green-v0.1.0.png"
      alt="R2 illustration in Launch Green"
      title="Personal Launch Green illustration; not a live image or paint reading."
      className={className}
    />
  );
}
