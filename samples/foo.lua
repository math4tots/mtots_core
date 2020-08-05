
function foo()
    total = 0
    for i= 1, 100000000 do
        total = total + i
    end
    print('total = ' .. total)
end

foo()
