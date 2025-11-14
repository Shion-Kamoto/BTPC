// Sample Supabase Edge Function
// This demonstrates a basic serverless function for BTPC

import { serve } from "https://deno.land/std@0.168.0/http/server.ts"

serve(async (req) => {
  const { method } = req;

  // Handle CORS
  const corsHeaders = {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
  }

  // Handle preflight request
  if (method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    // Get request data if POST
    let data = null;
    if (method === 'POST') {
      data = await req.json();
    }

    // Sample response
    const response = {
      message: "Hello from BTPC Edge Function!",
      timestamp: new Date().toISOString(),
      method: method,
      data: data,
      btpc: {
        version: "1.0.0",
        network: "testnet"
      }
    };

    return new Response(
      JSON.stringify(response),
      {
        headers: {
          ...corsHeaders,
          'Content-Type': 'application/json'
        },
        status: 200
      },
    )
  } catch (error) {
    return new Response(
      JSON.stringify({ error: error.message }),
      {
        headers: {
          ...corsHeaders,
          'Content-Type': 'application/json'
        },
        status: 400
      },
    )
  }
})