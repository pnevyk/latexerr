#!/bin/sh

# run latex on test files

cd tests

for file in *.tex
do
    pdflatex -draftmode -interaction=nonstopmode "$file" > /dev/null 2>&1
done

# run latexerr

cargo build > /dev/null 2>&1

errors=0

for file in *.log
do
    name="$(basename "$file" .log)"
    expected="$name.expected"

    # sed is for removing colors
    ../target/debug/latexerr -- "$file" | sed -r "s/\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[mGK]//g" > temp

    # if actual output is not equal to expected
    if ! diff temp "$expected" > /dev/null
    then
        echo "Test $name failed"
        errors=$(($errors + 1))
    fi
done

# print results
echo
if [ $errors -eq 0 ]
then
    echo "All tests completed successfully"
elif [ $errors -eq 1 ]
then
    echo "There is 1 error"
else
    echo "There are $errors errors"
fi

# clean
rm temp
rm *.log
rm *.aux
