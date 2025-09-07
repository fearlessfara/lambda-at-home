/**
 * Prime Number Calculator Lambda Function
 * 
 * This function computes the first n prime numbers.
 * It accepts a 'count' parameter to specify how many primes to calculate.
 */

exports.handler = async (event, context) => {
    console.log('Prime calculator function invoked');
    console.log('Event:', JSON.stringify(event, null, 2));

    // Extract count from event body
    const count = event.count;
    
    // Validate input
    if (count === undefined || count === null) {
        throw new Error('Invalid input. Please provide a number >= 2');
    }
    
    if (count < 2) {
        throw new Error(`Invalid count: ${count}. Must be >= 2.`);
    }

    console.log(`Calculating first ${count} prime numbers...`);
    
    const startTime = Date.now();
    const primes = calculatePrimes(count);
    const endTime = Date.now();
    
    const calculationTime = endTime - startTime;
    
    console.log(`Found ${primes.length} primes in ${calculationTime}ms`);

    // Prepare response
    const response = {
        count: count,
        primes: primes,
        calculationTimeMs: calculationTime,
        timestamp: new Date().toISOString()
    };

    console.log('Response:', JSON.stringify(response, null, 2));
    
    return response;
};

/**
 * Calculate the first n prime numbers using the Sieve of Eratosthenes
 * @param {number} n - Number of primes to calculate
 * @returns {number[]} Array of the first n prime numbers
 */
function calculatePrimes(n) {
    if (n === 1) return [2];
    if (n === 2) return [2, 3];
    
    // Use Sieve of Eratosthenes for all counts
    // Estimate upper bound: n * ln(n) + n * ln(ln(n))
    // For very large numbers, add some extra buffer
    let upperBound;
    if (n <= 10) {
        upperBound = n * 10; // More generous for small numbers
    } else if (n <= 1000) {
        upperBound = Math.ceil(n * Math.log(n) + n * Math.log(Math.log(n)));
    } else {
        // For large numbers, use a more generous upper bound
        // The nth prime is approximately n * ln(n), so we need significantly more
        upperBound = Math.ceil(n * Math.log(n) * 1.5);
    }
    
    const sieve = new Array(upperBound + 1).fill(true);
    
    // Mark 0 and 1 as not prime
    sieve[0] = sieve[1] = false;
    
    // Sieve of Eratosthenes
    for (let i = 2; i * i <= upperBound; i++) {
        if (sieve[i]) {
            for (let j = i * i; j <= upperBound; j += i) {
                sieve[j] = false;
            }
        }
    }
    
    // Collect primes
    const primes = [];
    for (let i = 2; i <= upperBound && primes.length < n; i++) {
        if (sieve[i]) {
            primes.push(i);
        }
    }
    
    // If we didn't find enough primes, extend the search
    if (primes.length < n) {
        // Extend upper bound and continue searching
        let extendedBound = upperBound * 2;
        const extendedSieve = new Array(extendedBound + 1).fill(true);
        extendedSieve[0] = extendedSieve[1] = false;
        
        // Re-run sieve with extended bound
        for (let i = 2; i * i <= extendedBound; i++) {
            if (extendedSieve[i]) {
                for (let j = i * i; j <= extendedBound; j += i) {
                    extendedSieve[j] = false;
                }
            }
        }
        
        // Collect more primes
        for (let i = 2; i <= extendedBound && primes.length < n; i++) {
            if (extendedSieve[i]) {
                primes.push(i);
            }
        }
    }
    
    return primes.slice(0, n);
}

/**
 * Check if a number is prime (simple trial division)
 * @param {number} num - Number to check
 * @returns {boolean} True if prime, false otherwise
 */
function isPrime(num) {
    if (num < 2) return false;
    if (num === 2) return true;
    if (num % 2 === 0) return false;
    
    for (let i = 3; i * i <= num; i += 2) {
        if (num % i === 0) return false;
    }
    
    return true;
}
