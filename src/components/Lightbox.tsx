import { Thumb } from "./Thumb";

interface Props {
  path: string;
  onClose: () => void;
}

export function Lightbox({ path, onClose }: Props) {
  return (
    <div className="lightbox" onClick={onClose}>
      <Thumb path={path} maxSide={null} />
    </div>
  );
}
