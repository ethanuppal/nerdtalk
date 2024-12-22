export enum TextBoxType {
  Authored,
  Trailing,
}

export interface TextBoxProps {
  type: TextBoxType
  body: string
  timestamp: string
  author: string
}

export default function TextBox(props: TextBoxProps) {
  const { type, body, timestamp, author } = props

  if (type == TextBoxType.Authored) return (
    <div className="flex gap-2 p-2 text-white hover:bg-gray-600">
      <img src="/image.png" className={"w-10 rounded-full" + (author == "ethan" ? " invert" : "")} />

      <div>
        <p>
          <span className="font-bold">{author}</span>{' '}
          <time className="text-xs text-gray-400">{timestamp}</time>{' '}
        </p>
        <p>{body}</p>
      </div>
    </div>
  ) 
  else return (
    <div className="flex gap-2 px-2 hover:bg-gray-600 text-secondary-color hover:text-gray-400">
      <div className="grid h-5 w-10 place-content-center">
        <div className="h-4 text-center">
          <time className="box-content block text-xs">{timestamp}</time>
        </div>
      </div>
      <p className="!text-white">{body}</p>
    </div>
  ) 

}
