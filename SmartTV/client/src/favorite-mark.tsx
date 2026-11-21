export function FavoriteMark({marked}: {marked: boolean}) {
  return (
    <svg class={`favorite-mark ${marked ? "is-favorite" : ""}`} aria-hidden="true"
      viewBox="0 0 24 24" fill={marked ? "currentColor" : "none"} stroke="currentColor"
      stroke-width="1.7" stroke-linejoin="round">
      <path d="M12 2.6l2.9 5.9 6.6 1-4.8 4.6 1.2 6.5-5.9-3.1-5.9 3.1 1.2-6.5L2.5 9.5l6.6-1z" />
    </svg>
  );
}
