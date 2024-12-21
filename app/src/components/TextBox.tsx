interface TextBoxParams {
    messageBody: string;
    timestamp: string;
    slotnum?: number;
}

export default function TextBox(props: TextBoxParams) {
    const {messageBody, timestamp} = props;

    return <div className="bg-primary-color rounded border-none px-2 py-2">
        <p>{messageBody} {timestamp}</p>

        {/* Bro needs to float top right */}
        {/* <p>{timestamp}</p> */}
    </div>
};
