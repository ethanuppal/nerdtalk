export default function InputBox() {
  return (
    <form className="bg-primary-color flex gap-2 rounded border px-2 py-2">
      <button>&gt;</button>
      <input
        className="bg-transparent focus:outline-none"
        type="text"
        placeholder="Enter message..."
      />
    </form>
  );
}
