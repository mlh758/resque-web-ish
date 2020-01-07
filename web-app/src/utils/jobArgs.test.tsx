import { renderArguments } from './jobArgs';

it('displays NULL for null/undefined values', () => {
    expect(renderArguments([null])).toEqual('null');
});

it('suffixes a long array with ...', () => {
    const argList = [1,2,3,4,5,6,7,8,9,10,11,12];
    expect(renderArguments([argList])).toEqual('[1, 2, 3, 4, 5, 6, 7, 8, 9, 10...]');
});

it('displays an empty argument list as <none>', () => {
    expect(renderArguments([])).toEqual('<none>');
});

it('converts numbers to strings for display', () => {
    expect(renderArguments([1, 2])).toEqual('1, 2');
})
