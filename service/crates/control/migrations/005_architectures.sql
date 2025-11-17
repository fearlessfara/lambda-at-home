-- Add architectures column to functions table
-- AWS Lambda spec: Architectures array with one value (x86_64 or arm64), default x86_64
ALTER TABLE functions ADD COLUMN architectures TEXT NOT NULL DEFAULT '["x86_64"]';
