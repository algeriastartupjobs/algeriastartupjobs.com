import { FC, useCallback, useEffect, useState } from "react";
import { Input, InputProps } from "..";
import debounce from "lodash/debounce";

interface DebouncedValueInputProps extends InputProps {
  debounceValue?: number;
}

export const DebouncedValueInput: FC<DebouncedValueInputProps> = ({
  value,
  setValue,
  debounceValue = 500,
  ...props
}) => {
  const [localValue, setLocalValue] = useState(value);

  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  const debouncedSetValue = useCallback(
    debounce((newLocalValue: string) => setValue(newLocalValue), debounceValue),
    []
  );

  useEffect(() => {
    debouncedSetValue(localValue);
  }, [localValue]);

  return <Input value={localValue} setValue={setLocalValue} {...props} />;
};
