const fib = num => {
	if (num <= 2) {
		return num;
	}

	return fib(num - 1) + fib(num - 2);
};

const input = process.env.NUM;

export default fib(input);
