original_name=thefug
new_name=thefugbindev

cargo build

if [ $? -ne 0 ]; then
  exit 1
fi

mv target/debug/$original_name target/debug/$new_name

echo "Dev build complete!"
echo "Set an alias to $(pwd)/target/debug/$new_name named $new_name to use it with the generated thefug script"
echo
echo "Example: alias $new_name='$(pwd)/target/debug/$new_name'"
